use std::collections::HashMap;

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::UdpSocket;

use log::debug;

use opentelemetry::{
    global,
    trace::{TraceContextExt, TraceError, Tracer},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};

use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;

use crate::client::sequence::Sequence;
use crate::config::Configuration;
use crate::packet::Packet;

#[derive(Clone)]
pub struct Server {
    configuration: Configuration,
}

pub enum ServerState {
    Continue,
    Exit,
}

impl Server {
    pub fn new(configuration: Configuration) -> Self {
        Self { configuration }
    }

    async fn sequence_reassemble(
        self,
        mut packet_rx: Receiver<Packet>,
        receiver: Sender<Sequence>,
    ) {
        let mut sequence_capture: HashMap<i64, Sequence> = HashMap::new();

        loop {
            let f = packet_rx.recv().await;
            match f {
                d => match d {
                    None => {}
                    Some(packet) => {
                        let sequence_id = packet.get_sequence_id().clone();
                        if packet.get_bytes_len() == 0 {
                            let sequence = sequence_capture.get_mut(&sequence_id).unwrap();
                            sequence
                                .packets
                                .sort_by(|a, b| a.packet_num.cmp(&b.packet_num));
                            if !sequence.is_complete() {
                                panic!("Sequence is not ordered correctly");
                            }
                            receiver
                                .send(Sequence {
                                    sequence_id,
                                    total_size: sequence.total_size,
                                    packets: sequence.packets.clone(),
                                })
                                .await
                                .unwrap();
                        }
                        if sequence_id != 0 {
                            if sequence_capture.contains_key(&sequence_id) {
                                sequence_capture
                                    .get_mut(&sequence_id)
                                    .unwrap()
                                    .packets
                                    .push(packet);
                            } else {
                                let packet = packet.clone();
                                sequence_capture.insert(
                                    sequence_id,
                                    Sequence {
                                        sequence_id,
                                        total_size: packet.sequence_len,
                                        packets: vec![packet],
                                    },
                                );
                            }
                        }
                    }
                },
            }
        }
    }

    fn init_tracer(&mut self) -> Result<opentelemetry_sdk::trace::Tracer, TraceError> {
        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint("http://localhost:4317"),
            )
            .with_trace_config(
                sdktrace::config()
                    .with_resource(Resource::new(vec![KeyValue::new(SERVICE_NAME, "adrnaln")])),
            )
            .install_batch(runtime::Tokio)
    }
    pub async fn start(&mut self, mut kill_rx: oneshot::Receiver<i64>) {
        // Enable tracing
        global::set_text_map_propagator(opentelemetry_jaeger_propagator::Propagator::new());
        let _tracer = self.init_tracer().expect("Failed to initialize tracer.");
        let tracer = global::tracer("server-tracer");

        let _root_span = tracer.start("root-span");
        // start the async reassembling
        let (packet_tx, packet_rx) = mpsc::channel::<Packet>(1000);
        spawn({
            let me = self.clone();
            let me_too = self.clone();
            async move {
                // move the sequence_rx ownership
                me.sequence_reassemble(packet_rx, me_too.configuration.sequence_tx)
                    .await;
            }
        });
        let socket = self.new_udp_reuse_port(self.configuration.addresses.local_address);
        loop {
            let mut buf = [0; 9134];
            let _me = self.clone();
            let packet_tx = packet_tx.clone();

            match kill_rx.try_recv() {
                Ok(_) => {
                    debug!("Killing server");
                    tokio::task::spawn_blocking(opentelemetry::global::shutdown_tracer_provider)
                        .await
                        .expect("Error shutting down tracing provider");
                    return;
                }
                Err(_) => {}
            }
            let s = socket.recv_from(&mut buf);
            match s {
                Ok((len, _)) => {
                    if len > 0 {
                        tokio::spawn({
                            let buf = buf.clone();
                            let tracer = global::tracer("server-tracer");
                            async move {
                                tracer
                                    .in_span("packet-received", |cx| async move {
                                        let span = cx.span();
                                        let pk = Packet::from_bytes(buf);
                                        span.set_attribute(KeyValue::new(
                                            "packet_num",
                                            pk.packet_num,
                                        ));
                                        span.set_attribute(KeyValue::new(
                                            "sequence_id",
                                            pk.sequence_id,
                                        ));
                                        packet_tx.send(pk).await.unwrap();
                                    })
                                    .await;
                            }
                        })
                        .await
                        .expect("Could not spawn packet send");
                    }
                }
                Err(e) => {
                    if !matches!(e.kind(), ErrorKind::WouldBlock) {
                        continue;
                    }
                }
            }
            sleep(self.clone().configuration.server_sleep_time()).await;
        }
    }
    fn new_udp_reuse_port(&self, local_addr: SocketAddr) -> UdpSocket {
        let udp_sock = socket2::Socket::new(
            if local_addr.is_ipv4() {
                socket2::Domain::IPV4
            } else {
                socket2::Domain::IPV6
            },
            socket2::Type::DGRAM,
            None,
        )
        .unwrap();
        udp_sock.set_reuse_port(true).unwrap();
        udp_sock.set_cloexec(true).unwrap();
        udp_sock
            .set_nonblocking(true)
            .expect("Failed to set non-blocking");
        udp_sock.bind(&socket2::SockAddr::from(local_addr)).unwrap();
        let udp_sock: std::net::UdpSocket = udp_sock.into();
        udp_sock.try_into().unwrap()
    }
}

#[cfg(test)]
mod test {
    use log::debug;

    use std::time::Duration;

    use tokio::spawn;
    use tokio::sync::{mpsc, oneshot};

    use crate::client::sequence::Sequence;
    use crate::client::Client;
    use crate::config::Addresses;
    use crate::server::{Configuration, Server};

    #[tokio::test]
    async fn test_transmit_sequence() {
        let la = "0.0.0.0:8087".parse().unwrap();
        let ra = "0.0.0.0:8087".parse().unwrap();
        let (sequence_tx, mut sequence_rx) = mpsc::channel::<Sequence>(100);
        let (kill_tx, kill_rx) = oneshot::channel();
        let mut server = Server::new(Configuration {
            addresses: Addresses {
                local_address: la,
                remote_address: ra,
            },
            sequence_tx,
        });

        let client = Client::new(Addresses {
            local_address: "0.0.0.0:0".parse().unwrap(),
            remote_address: la,
        });

        spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let seq = client
                .build_sequence_from_file("examples/resources/fox.png")
                .await
                .expect("File not loaded");
            client.send_sequence(seq).await.unwrap();

            match sequence_rx.recv().await {
                None => {
                    panic!("Sequence not received");
                }
                Some(_) => {
                    debug!("Sequence received");
                    kill_tx.send(1).unwrap();
                }
            }
        });

        server.start(kill_rx).await;
    }
}
