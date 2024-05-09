use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::UdpSocket;

use crate::client::sequence::Sequence;
use crate::config::Configuration;
use crate::packet::Packet;
use std::sync::{Arc, Mutex};
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use tracing::{info, instrument};

#[derive(Clone, Debug)]
pub struct Server {
    configuration: Configuration,
}

pub enum ServerState {
    Continue,
    Exit,
}

impl Server {
    #[instrument]
    pub fn new(configuration: Configuration) -> Self {
        Self { configuration }
    }

    #[instrument(skip_all)]
    async fn assemble(
        self,
        packet: Packet,
        sequence_capture: Arc<Mutex<HashMap<i64, Sequence>>>,
    ) -> Option<Sequence> {
        let sequence_id = packet.get_sequence_id().clone();
        let mut sequence_db = sequence_capture.lock().unwrap();
        let sequence = sequence_db.get_mut(&sequence_id);
        match sequence {
            None => {
                // Let's add that sequence in
                info!("Inserting Packet {} into new Sequence {}", packet.packet_num, sequence_id);
                sequence_db.insert(
                    sequence_id,
                    Sequence {
                        sequence_id,
                        total_size: packet.sequence_len,
                        packets: vec![packet],
                    },
                );
            }
            Some(found_sequence) => {
                if packet.get_bytes_len() == 0 {
                    info!("Completed Sequence {}", found_sequence.sequence_id);
                    found_sequence
                        .packets
                        .sort_by(|a, b| a.packet_num.cmp(&b.packet_num));
                    if !found_sequence.is_complete() {
                        panic!("Sequence is not ordered correctly");
                    }
                    return Some(Sequence {
                        sequence_id,
                        total_size: found_sequence.total_size,
                        packets: found_sequence.packets.clone(),
                    });
                }
                found_sequence.packets.push(packet);
            }
        }
        None
    }
    #[instrument(skip_all)]
    async fn start_sequence_reassemble(
        self,
        mut packet_rx: Receiver<Packet>,
        receiver: Sender<Sequence>,
    ) {
        let sequence_capture: Arc<Mutex<HashMap<i64, Sequence>>> =
            Arc::new(Mutex::new(HashMap::new()));
        loop {
            let f = packet_rx.recv().await;
            match f {
                d => match d {
                    None => {}
                    Some(packet) => {
                        let sequence = sequence_capture.clone();
                        let me = self.clone();
                        let maybe_sequence = me.assemble(packet, sequence).await;
                        match maybe_sequence {
                            None => {}
                            Some(sequence) => {
                                receiver.send(sequence).await.unwrap();
                            }
                        }
                    }
                },
            }
        }
    }

    #[instrument(skip_all)]
    pub async fn process_buffer(&mut self, packet_tx: Sender<Packet>, buf: [u8; 9134]) {
        let pk = Packet::from_bytes(buf);
        info!("Sending Packet {} of sequence {}", pk.packet_num, pk.sequence_id);
        packet_tx.send(pk).await.unwrap();
    }
    #[instrument]
    pub async fn start(&mut self, mut kill_rx: oneshot::Receiver<i64>) {
        // start the async reassembling
        let (packet_tx, packet_rx) = mpsc::channel::<Packet>(1000);
        spawn({
            let me = self.clone();
            let me_too = self.clone();
            async move {
                // move the sequence_rx ownership
                me.start_sequence_reassemble(packet_rx, me_too.configuration.sequence_tx)
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
                    info!("Killing server");
                    return;
                }
                Err(_) => {}
            }
            let s = socket.recv_from(&mut buf);
            match s {
                Ok((len, _)) => {
                    if len > 0 {
                        spawn({
                            let mut me = self.clone();
                            async move {
                                me.process_buffer(packet_tx, buf).await;
                            }
                        });
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
    #[instrument]
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
