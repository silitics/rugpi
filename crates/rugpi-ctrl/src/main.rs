use rugpi_common::Anyhow;

pub mod cli;
pub mod init;
pub mod overlay;
pub mod state;
pub mod utils;

pub fn main() -> Anyhow<()> {
    if utils::is_init_process() {
        init::main()
    } else {
        cli::main()
    }
}
