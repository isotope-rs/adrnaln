pub mod sequence;
use crate::client::sequence::Sequence;
use crate::config::Addresses;
use crate::packet::{Packet, ACTUAL_BUFFER_SIZE};
use log::debug;
use rand::Rng;
use std::error::Error;
use std::io::Read;
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::net::UdpSocket;

pub struct Client {
    addresses: Addresses,
}

impl Client {
    pub fn new(addresses: Addresses) -> Self {
        Self { addresses }
    }
    pub async fn send_packet(&self, packet: Packet) -> Result<(), Box<dyn Error + Send>> {
        let socket = UdpSocket::bind(self.addresses.local_address).await.unwrap();
        socket
            .connect(&self.addresses.remote_address)
            .await
            .unwrap();
        if packet.get_bytes_len() < 100 {
            debug!("{}", packet.bytes.len())
        }
        socket.send(&packet.to_bytes()).await.unwrap();
        Ok(())
    }

    pub async fn send_sequence(&self, sequence: Sequence) -> Result<(), Box<dyn Error + Send>> {
        for member in sequence.packets {
            self.send_packet(member).await?
        }
        Ok(())
    }
    pub async fn build_sequence_from_file(
        &self,
        filename: &str,
    ) -> Result<Sequence, Box<dyn Error>> {
        let seq_n: i64 = rand::thread_rng().gen_range(1000..100000);
        let path = Path::new(filename);
        let name = path.file_name().unwrap();
        let mut sequence = Sequence {
            sequence_id: i64::from(seq_n),
            total_size: 0,
            packets: vec![],
        };
        let mut list_of_chunks = Vec::new();
        let mut f = std::fs::File::open(filename)?;
        loop {
            let mut chunk = Vec::with_capacity(ACTUAL_BUFFER_SIZE);
            let n = std::io::Read::by_ref(&mut f)
                .take(ACTUAL_BUFFER_SIZE as u64)
                .read_to_end(&mut chunk)?;
            if n == 0 {
                break;
            }
            sequence.total_size += n;
            list_of_chunks.push(chunk);
        }
        let mut packet_counter = 0;
        for chunk in list_of_chunks {
            let p = Packet {
                packet_num: packet_counter,
                sequence_id: seq_n,
                filename: name.to_str().unwrap().parse().unwrap(),
                bytes: chunk,
                sequence_len: sequence.total_size,
            };
            sequence.packets.push(p);
            packet_counter = packet_counter + 1;
        }
        // We send an empty sequenced packet here to indicate a sign-off
        sequence.packets.push(Packet {
            sequence_id: seq_n,
            packet_num: packet_counter,
            sequence_len: sequence.total_size,
            filename: name.to_str().unwrap().parse().unwrap(),
            bytes: vec![],
        });
        Ok(sequence)
    }
}
