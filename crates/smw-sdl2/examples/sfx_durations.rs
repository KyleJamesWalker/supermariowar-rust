//! Prints the sound-duration table for the repository's data/.
//! cargo run -p smw-sdl2 --example sfx_durations > crates/smw-core/src/common/sfx_durations.txt

fn main() {
    let data = std::env::args().nth(1).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../data").to_string());
    print!("{}", smw_sdl2::durations::measure(std::path::Path::new(&data)));
}
