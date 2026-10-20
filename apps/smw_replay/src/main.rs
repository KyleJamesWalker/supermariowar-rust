//! `smw_replay`: the game on the headless backend.

fn main() {
    smw::smw::main::run_with(std::env::args().collect(), smw_replay::services());
}
