#[cfg(test)]
mod test {
    use log::debug;

    use std::time::Duration;

    use adrnaln::client::sequence::Sequence;
    use adrnaln::client::Client;
    use adrnaln::config::{Addresses, Configuration};
    use adrnaln::packet;
    use adrnaln::server::Server;
    use tokio::spawn;
    use tokio::sync::{mpsc, oneshot};

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
                .build_sequence_from_file("images/arch.png")
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

    #[tokio::test]
    async fn test_server_ipv4() {
        let la = "0.0.0.0:8085".parse().unwrap();
        let ra = "0.0.0.0:8085".parse().unwrap();
        let (sequence_tx, mut sequence_rx) = mpsc::channel(100);
        let (kill_tx, _kill_rx) = oneshot::channel();
        let _server = Server::new(Configuration {
            addresses: Addresses {
                local_address: la,
                remote_address: ra,
            },
            sequence_tx,
        });
        // client send
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let client = Client::new(Addresses {
                local_address: ra,
                remote_address: la,
            });
            client
                .send_packet(packet::Packet {
                    packet_num: 1,
                    sequence_id: 1,
                    sequence_len: 1,
                    filename: "none".to_string(),
                    bytes: vec![1],
                })
                .await
                .unwrap();
        });

        tokio::spawn(async move {
            loop {
                match sequence_rx.recv().await {
                    None => {}
                    Some(sequence) => {
                        assert_eq!(sequence.sequence_id, 1);
                        assert_eq!(sequence.total_size, 1);
                        assert_eq!(sequence.packets.len(), 1);
                        assert_eq!(sequence.packets[0].packet_num, 1);
                        assert_eq!(sequence.packets[0].sequence_id, 1);
                        assert_eq!(sequence.packets[0].sequence_len, 1);
                        assert_eq!(sequence.packets[0].bytes.len(), 1);
                        kill_tx.send(1).unwrap();
                        break;
                    }
                }
            }
        });
    }
}
