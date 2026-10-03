//! Port of src/common/CmdArgs.cpp

#[derive(Clone, Debug, Default)]
pub struct Args {
    pub success: bool,
    pub show_help: bool,
    pub debug: bool,
    pub data_root: String,
}

pub fn show_windows_console() {}

pub fn print_help(title: &str, version: &str) {
    print!("{} {}\n\n", title, version);
    print!(
        "Super Mario War is a Super Mario multiplayer game. The goal is to stomp\n\
         as many other Marios as possible to win the game. It's a tribute to Nintendo\n\
         and the game Mario War by Samuele Poletto.\n\n"
    );
    println!("Options:");
    println!("  -h, --help              Prints this help");
    println!("      --datadir <DIR>     Sets the data directory to DIR (default: ./data)");
    println!("      --debug             Shows the debug console on Windows");
}

pub fn parse_args(argv: &[String]) -> Args {
    let mut result = Args::default();
    let argc = argv.len();
    let mut i = 1;
    while i < argc {
        let arg = argv[i].as_str();
        if arg == "-h" || arg == "--help" {
            result.success = true;
            result.show_help = true;
            return result;
        }
        if arg == "--debug" {
            result.debug = true;
            i += 1;
            continue;
        }
        if arg == "--datadir" {
            i += 1;
            if i >= argc {
                eprintln!("Error: `--datadir` requires a parameter, see `--help`");
                return result;
            }
            result.data_root = argv[i].clone();
            i += 1;
            continue;
        }
        i += 1;
    }
    result.success = true;
    result
}
