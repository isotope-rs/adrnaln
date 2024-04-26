use std::path::Path;
use tokio::fs;
use adrnaln::client::sequence::Sequence;

pub async fn write_sequence_to_file(sequence: Sequence) {
    let mut bytes = vec![];
    let mut filename = "".to_string();
    for packets in &sequence.packets {
        bytes.extend(packets.clone().bytes);
        if filename.is_empty() {
            filename = packets.filename.clone();
        }
    }
    let path = Path::new(&filename);
    fs::write(path, &bytes)
        .await
        .expect("Could not write file!");
}