use rugix_blocking::blocking;
use rugix_fs::Copier;

#[tokio::main]
async fn main() {
    let mut args = std::env::args();
    args.next();
    let src = args.next().unwrap();
    let dst = args.next().unwrap();
    blocking(move |cx| {
        let mut copier = Copier::new();
        copier.copy_dir(cx, src.as_ref(), dst.as_ref())
    })
    .await
    .unwrap();
}
