pub mod cli;
pub mod config;
pub mod init;
pub mod partitions;
pub mod state;

pub fn main() -> anyhow::Result<()> {
    if init::is_init_process() {
        init::main()
    } else {
        cli::main()
    }
}
