use rugix_fs::Copier;
use rugix_tasks::run_blocking_unchecked;

fn main() {
    let mut args = std::env::args();
    args.next();
    let src = args.next().unwrap();
    let dst = args.next().unwrap();
    run_blocking_unchecked(move |cx| {
        let mut copier = Copier::new();
        copier.copy_dir(cx, src.as_ref(), dst.as_ref())
    })
    .unwrap()
    .unwrap();
}
