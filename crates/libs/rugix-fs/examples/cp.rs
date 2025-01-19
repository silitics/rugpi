use rugix_fs::{Copier, FsResult};
use rugix_tasks::spawn_blocking;

fn main() -> FsResult<()> {
    let mut args = std::env::args();
    args.next();
    let src = args.next().unwrap();
    let dst = args.next().unwrap();
    spawn_blocking(move || {
        let mut copier = Copier::new();
        copier.copy_dir(src.as_ref(), dst.as_ref())
    })
    .join_blocking()
}
