//! Prints SDL2's key table for the headless backend.
//! cargo run -p smw-sdl2 --example sdl2_keys > crates/smw-headless/src/sdl2_keys.txt

fn main() {
    print!("{}", smw_sdl2::keymap::measure());
}
