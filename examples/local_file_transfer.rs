use clap::Parser;

use adrnaln::config::Addresses;
use adrnaln::{client::Client, config::Configuration, server::Server};
use opentelemetry::global;
use std::time::Duration;
use tokio;
use tokio::spawn;
use tokio::sync::{mpsc, oneshot};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    filepath: String,
}
mod helper;

#[tokio::main]
async fn main() {
    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
    // Sets up the machinery needed to export data to Jaeger
    // There are other OTel crates that provide pipelines for the vendors
    // mentioned earlier.
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("adrnaln")
        .install_simple()
        .unwrap();

    // Create a tracing layer with the configured tracer
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // The SubscriberExt and SubscriberInitExt traits are needed to extend the
    // Registry to accept `opentelemetry (the OpenTelemetryLayer type).
    tracing_subscriber::registry()
        .with(opentelemetry)
        // Continue logging to stdout
        .with(fmt::Layer::default())
        .try_init()
        .unwrap();
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

        let client = Client::new(Addresses {
            local_address: "0.0.0.0:0".parse().unwrap(),
            remote_address: la,
        });
        let sequence = client
            .build_sequence_from_file(args.filepath.as_str())
            .await
            .unwrap();

        client.send_sequence(sequence).await.unwrap();
        match sequence_rx.recv().await {
            None => {}
            Some(sequence) => {
                helper::write_sequence_to_file("examples", sequence).await;
            }
        }
        kill_tx.send(1).unwrap();
    });

    server.start(kill_rx).await;
}
