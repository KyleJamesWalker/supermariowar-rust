fn main() {
    smw::worldeditor::worldeditor::main_with(std::env::args().collect(), smw_replay::services());
}
