use crate::client::sequence::Sequence;

use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub struct Configuration {
    pub addresses: Addresses,
    pub sequence_tx: Sender<Sequence>,
}

impl Configuration {
    pub fn server_sleep_time(self) -> Duration {
        Duration::from_millis(5)
    }
}

#[derive(Clone, Debug)]
pub struct Addresses {
    pub local_address: SocketAddr,
    pub remote_address: SocketAddr,
}
