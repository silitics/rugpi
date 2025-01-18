use std::time::Duration;

use rugix_tasks::{
    check_abort, run_blocking_unchecked, shutdown_blocking, spawn, spawn_blocking, BlockingCtx,
    MaybeAborted,
};
use rugix_try::xtry;

/// Does some work indefinitely until the task is aborted.
pub fn work_indefinitely(cx: BlockingCtx) -> MaybeAborted<()> {
    fn some_work(cx: BlockingCtx) -> MaybeAborted<()> {
        check_abort!(cx);
        eprintln!("doing some work...");
        std::thread::sleep(Duration::from_secs(10));
        MaybeAborted::Done(())
    }

    loop {
        xtry!(some_work(cx))
    }
}

pub fn main() {
    run_blocking_unchecked(|ctx| {
        spawn(async {
            // Let's race the indefinite blocking work against an asynchronous timeout.
            let blocking_task = spawn_blocking(|ctx| work_indefinitely(ctx));
            tokio::select! {
                _ = blocking_task.join() => {
                    // This will never happen, because the work takes forever.
                    unreachable!("work done!")
                }
                _ = tokio::time::sleep(Duration::from_secs(15)) => {
                    eprintln!("timeout: aborting task");
                }
            }
        })
        .join_blocking(ctx);
        eprintln!("main task terminated, waiting on background tasks");
        // Dropping the runtime will wait for background tasks to finish.
        shutdown_blocking(ctx);
        eprintln!("all tasks have terminated");
    });
}
