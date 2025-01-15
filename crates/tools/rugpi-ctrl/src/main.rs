pub mod cli;
pub mod init;
pub mod overlay;
pub mod state;
pub mod utils;

pub fn main() {
    let result = if utils::is_init_process() {
        init::main()
    } else {
        cli::main()
    };
    if let Err(report) = result {
        eprintln!("{report:?}");
        std::process::exit(1);
    }
}
