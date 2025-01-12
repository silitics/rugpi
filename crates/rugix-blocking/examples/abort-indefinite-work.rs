use std::time::Duration;

use rugix_blocking::{blocking, Aborted, BlockingCtx};

/// Does some work indefinitely until the task is aborted.
pub fn infinite_work<'cx>(cx: BlockingCtx<'cx>) -> Result<(), Aborted<'cx>> {
    loop {
        cx.check_aborted()?;
        eprintln!("doing some work...");
        std::thread::sleep(Duration::from_secs(10));
    }
}

pub fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        // Let's race the indefinite blocking work against an asynchronous timeout.
        tokio::select! {
            _ = blocking(|cx| infinite_work(cx)) => {
                // This will never happen, because the work takes forever.
                unreachable!("work done!")
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                eprintln!("timeout: aborting task");
            }
        }
    });
    eprintln!("main task terminated, waiting on background tasks");
    // Dropping the runtime will wait for background tasks to finish.
    drop(runtime);
    eprintln!("everything has been aborted");
}
