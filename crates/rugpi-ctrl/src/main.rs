use rugpi_common::Anyhow;

pub mod cli;
pub mod init;
pub mod overlay;
pub mod state;

pub fn main() -> Anyhow<()> {
    if init::is_init_process() {
        init::main()
    } else {
        cli::main()
    }
}
