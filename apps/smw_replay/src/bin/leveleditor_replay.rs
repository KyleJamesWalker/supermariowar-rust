//! `leveleditor_replay`: the level editor on the headless backend.

fn main() {
    smw::leveleditor::leveleditor::main_with(std::env::args().collect(), smw_replay::services());
}
