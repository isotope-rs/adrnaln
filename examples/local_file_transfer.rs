use clap::Parser;
use opentelemetry::global;

use std::time::Duration;
use tokio;
use tokio::sync::{mpsc, oneshot};
use tokio::{spawn};
use tracing::{span, Level};

use adrnaln::config::Addresses;
use adrnaln::{client::Client, config::Configuration, server::Server};
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    filepath: String,
}
mod helper;
#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    // In this example we will use the server and client in the same code
    // Sending a file to ourselves
    let la = "0.0.0.0:8085".parse().unwrap();
    let ra = "0.0.0.0:8085".parse().unwrap();
    let (sequence_tx, mut sequence_rx) = mpsc::channel(100);
    let (kill_tx, kill_rx) = oneshot::channel();
    let mut server = Server::new(Configuration {
        addresses: Addresses {
            local_address: la,
            remote_address: ra,
        },
        sequence_tx,
    });
    spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _tracer = global::tracer("client-tracer");
        let client = Client::new(Addresses {
            local_address: "0.0.0.0:0".parse().unwrap(),
            remote_address: la,
        });
        let span = span!(Level::DEBUG, "build-sequence");
        let _enter = span.enter();

        let sequence = client
            .build_sequence_from_file(args.filepath.as_str())
            .await
            .unwrap();
        let span = span!(Level::DEBUG, "send-sequence");
        let _enter = span.enter();
        client.send_sequence(sequence).await.unwrap();

        match sequence_rx.recv().await {
            None => {}
            Some(sequence) => {
                helper::write_sequence_to_file(sequence).await;
            }
        }
        kill_tx.send(1).unwrap();
    });

    server.start(kill_rx).await;
}
