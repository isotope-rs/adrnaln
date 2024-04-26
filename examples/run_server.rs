use adrnaln::config::{Addresses, Configuration};
use adrnaln::server::Server;
use signal_hook::{consts::SIGINT, iterator::Signals};
use tokio::sync::{mpsc, oneshot};

use log::debug;

#[tokio::main]
async fn main() {
    let la = "0.0.0.0:8085".parse().unwrap();
    let ra = "0.0.0.0:8085".parse().unwrap();
    let (sequence_tx, _sequence_rx) = mpsc::channel(100);
    let (kill_tx, kill_rx) = oneshot::channel();
    let mut server = Server::new(Configuration {
        addresses: Addresses {
            local_address: la,
            remote_address: ra,
        },
        sequence_tx,
    });

    let mut signals = Signals::new(&[SIGINT]).unwrap();

    tokio::spawn(async move {
        for _sig in signals.forever() {
            debug!("Exiting server");
            break;
        }
        kill_tx.send(1).unwrap()
    });

    server.start(kill_rx).await;
}
