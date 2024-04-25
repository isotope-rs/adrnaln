use serde::{Deserialize, Serialize};
pub(crate) const MAX_BUFFER_SIZE: usize = 9126;
pub(crate) const SEQUENCE_SIZE: usize = 8;

pub(crate) const PACKET_NUM_SIZE: usize = 8;
pub(crate) const FILENAME_SIZE: usize = 255;
pub(crate) const ACTUAL_BUFFER_SIZE: usize =
    MAX_BUFFER_SIZE - SEQUENCE_SIZE - FILENAME_SIZE - PACKET_NUM_SIZE;
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Packet {
    pub packet_num: i64,
    pub sequence_id: i64,
    pub sequence_len: usize,
    pub filename: String,
    pub bytes: Vec<u8>,
}

impl Packet {
    pub fn get_bytes_len(&self) -> usize {
        self.bytes.len()
    }
    pub fn from_bytes(raw_bytes: [u8; 9134]) -> Self {
        let decoded: Packet = bincode::deserialize(&raw_bytes[..]).unwrap();
        Self {
            packet_num: decoded.packet_num,
            sequence_id: decoded.sequence_id,
            filename: decoded.filename,
            sequence_len: decoded.sequence_len,
            bytes: decoded.bytes,
        }
    }
    pub fn get_packet_num(&self) -> i64 {
        return self.packet_num;
    }
    pub fn get_sequence_id(&self) -> i64 {
        return self.sequence_id;
    }
    pub fn to_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}
