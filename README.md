# adrnaln

Problem statement: It is desired to send any type of file across UDP with low latency or metadata overhead.

```
This library is a toy example of how that might be accomplished.
It is a toy, as it does not implement retry mechanisms, backoff, telemetry etc..
```

## Implementation

<img src="images/arch.png" width="750px;" />

This library uses `bincode` to assist in the serial/deseralisation of `Packet` structs.
These structs contain chunks from input file and also have global metadata about the sequence construction.

## Usage

`cargo test` to run a synthetic test.

Run the example `cargo run --example send_file -- --filepath <FILEPATH>` ( This will currently spit the file into the cwd )

### Performance

To analyze performance use `flamegraph` ( cargo install flamegraph )

```
sudo cargo flamegraph --dev --example send_file -- --filepath <FILEPATH>
```

<img src="images/flamegraph.svg" width="450px;" />
