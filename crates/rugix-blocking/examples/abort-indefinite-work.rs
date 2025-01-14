use std::time::Duration;

use rugix_blocking::{block, blocking, check_abort, BlockingCtx, MaybeAborted};

/// Does some work indefinitely until the task is aborted.
pub fn work_indefinitely<'cx>(cx: BlockingCtx<'cx>) -> MaybeAborted<'cx, ()> {
    fn some_work<'cx>(cx: BlockingCtx<'cx>) -> MaybeAborted<'cx, ()> {
        check_abort!(cx);
        eprintln!("doing some work...");
        std::thread::sleep(Duration::from_secs(10));
        MaybeAborted::Done(())
    }

    loop {
        block!(some_work(cx))
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
            _ = blocking(|cx| work_indefinitely(cx)) => {
                // This will never happen, because the work takes forever.
                unreachable!("work done!")
            }
            _ = tokio::time::sleep(Duration::from_secs(15)) => {
                eprintln!("timeout: aborting task");
            }
        }
    });
    eprintln!("main task terminated, waiting on background tasks");
    // Dropping the runtime will wait for background tasks to finish.
    drop(runtime);
    eprintln!("all tasks have terminated");
}
