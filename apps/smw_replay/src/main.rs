fn main() {
    smw::smw::main::run_with(std::env::args().collect(), smw_replay::services());
}
