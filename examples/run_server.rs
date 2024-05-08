mod helper;
use signal_hook::{consts::SIGINT, iterator::Signals};
use tokio::sync::{mpsc, oneshot};

use adrnaln::config::Addresses;
use adrnaln::{config::Configuration, server::Server};
use opentelemetry::global;

use tokio;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
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

    let mut signals = Signals::new(&[SIGINT]).unwrap();

    tokio::spawn(async move {
        for _sig in signals.forever() {
            break;
        }
        kill_tx.send(1).unwrap()
    });

    tokio::spawn(async move {
        loop {
            match sequence_rx.recv().await {
                None => {}
                Some(sequence) => {
                    helper::write_sequence_to_file(".", sequence).await;
                }
            }
        }
    });
    server.start(kill_rx).await;
}
