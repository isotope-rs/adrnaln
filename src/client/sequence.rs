use crate::packet::Packet;

pub struct Sequence {
    pub sequence_id: i64,
    pub total_size: usize,
    pub packets: Vec<Packet>,
}

impl Sequence {
    pub fn is_complete(&self) -> bool {
        // rewind through all packets in the sequence to identify if there are missing packets
        let count = self.packets.len();
        if count == 0 {
            return false;
        }
        let mut x = self.packets[count - 1].packet_num;
        while x > 0 {
            let p = self.packets[x as usize].clone();
            if p.packet_num != x {
                return false;
            }
            x = x - 1;
        }
        true
    }
}
