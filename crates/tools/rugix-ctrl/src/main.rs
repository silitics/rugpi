pub mod cli;
pub mod config;
pub mod http_source;
pub mod init;
pub mod overlay;
pub mod slot_db;
pub mod state;
pub mod system_state;
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
