//! Prints the sound-duration table (crates/smw-core/src/common/sfx_durations.rs) for the repository's data/.
//! cargo run --example sfx_durations > crates/smw-core/src/common/sfx_durations.txt

fn main() {
    let data = std::env::args().nth(1).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../data").to_string());
    print!("{}", smw::common::sfx_durations::measure(std::path::Path::new(&data)));
}
