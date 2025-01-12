//! Functionality for running _abortable, blocking tasks_ in an asynchronous context.
//!
//! This crate provides a type [`BlockingCtx`] representing a context in which blocking
//! work can be performed. This type serves two purposes: (1) We use it as a marker to
//! indicate which functions perform blocking work, preventing them from being
//! accidentally called in an asynchronous context, and (2) it is used to signal to a
//! blocking task when it should be aborted.
//!
//! [`BlockingCtx`] is `!Send`, `!Sync`, and has a `'cx` lifetime brand, preventing it
//! from ever leaving the scope in which the work is performed. The only way to create a
//! blocking context is by invoking [`blocking`], where each invocation will result in a
//! uniquely-branded context. The type [`Aborted`] is used to signal when a task has
//! been aborted. It is also lifetime-branded to prevent it from being used with a
//! different blocking task than the one which should be aborted. The only way to create
//! [`Aborted`] with the correct brand is to use [`BlockingCtx::check_aborted`], which
//! checks whether the task has been aborted.
//!
//! In the future, we can also easily extend [`BlockingCtx`] to provide progress updates
//! for blocking tasks.
//!
//! Note that there would be a need for something like this even if Rugix would not use
//! any asynchronous code at all, as we would like to gracefully cancel operations (like
//! the creation of layers in Rugix Bakery) in any case. An alternative, maybe simpler
//! design would use a global signal to terminate all running tasks. This would, however,
//! not be as flexible.

use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::task::Poll;

/// Abortable, blocking task running in the background on a thread pool.
///
/// Dropping this structure will abort the task.
pub struct BlockingTask<T> {
    /// Handle obtained from [`tokio::task::spawn_blocking`].
    ///
    /// We use an [`Option`] here as we need to take out the handle in
    /// [`BlockingTask::abort`].
    handle: Option<tokio::task::JoinHandle<Result<T, Aborted<'static>>>>,
    /// Shared task data.
    shared: Arc<BlockingTaskShared>,
}

impl<T> BlockingTask<T> {
    /// Abort the task.
    ///
    /// The returned future can be used to wait until the task is aborted.
    pub fn abort(mut self) -> AbortedTask<T> {
        AbortedTask {
            handle: self.handle.take().unwrap(),
        }
    }
}

impl<T> Debug for BlockingTask<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockingTask")
            .field("task_id", &self.handle.as_ref().unwrap().id())
            .field("should_abort", &self.shared.should_abort())
            .finish()
    }
}

impl<T> Future for BlockingTask<T> {
    type Output = T;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let handle = Pin::new(self.handle.as_mut().unwrap());
        let Poll::Ready(value) = handle.poll(cx) else {
            return Poll::Pending;
        };
        match value {
            Ok(Ok(value)) => Poll::Ready(value),
            Ok(Err(_)) => {
                // The task is only aborted when it is dropped. When that happens, the
                // future is never polled again. Hence, this is unreachable.
                unreachable!("`BlockingTask` has been aborted without being dropped?!?");
            }
            Err(error) => {
                // The Tokio task has been cancelled or panicked. As the task is a
                // blocking task, we are sure that it has not been cancelled. Thus,
                // it must have panicked. Let's propagate the panic.
                std::panic::resume_unwind(error.into_panic())
            }
        }
    }
}

impl<T> Drop for BlockingTask<T> {
    fn drop(&mut self) {
        self.shared.abort();
    }
}

/// Blocking task that has been aborted.
pub struct AbortedTask<T> {
    /// Handle obtained from [`tokio::task::spawn_blocking`].
    handle: tokio::task::JoinHandle<Result<T, Aborted<'static>>>,
}

impl<T> Debug for AbortedTask<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockingTask")
            .field("task_id", &self.handle.id())
            .finish()
    }
}

impl<T> Future for AbortedTask<T> {
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let handle = Pin::new(&mut self.handle);
        let Poll::Ready(value) = handle.poll(cx) else {
            return Poll::Pending;
        };
        match value {
            Ok(Ok(value)) => Poll::Ready(Some(value)),
            Ok(Err(_)) => Poll::Ready(None),
            Err(error) => {
                // The Tokio task has been cancelled or panicked. As the task is a
                // blocking task, we are sure that it has not been cancelled. Thus,
                // it must have panicked. Let's propagate the panic.
                std::panic::resume_unwind(error.into_panic())
            }
        }
    }
}

/// Shared task data.
#[derive(Debug, Default)]
struct BlockingTaskShared {
    /// Indicates whether the task should be aborted.
    should_abort: AtomicBool,
}

impl BlockingTaskShared {
    /// Abort the task.
    fn abort(&self) {
        self.should_abort.store(true, atomic::Ordering::Relaxed);
    }

    /// Check whether the task should be aborted.
    fn should_abort(&self) -> bool {
        self.should_abort.load(atomic::Ordering::Relaxed)
    }
}

/// Context corresponding to a [`BlockingTask`] in which blocking work can be performed.
#[derive(Debug, Clone, Copy)]
pub struct BlockingCtx<'cx> {
    /// Shared task data.
    shared: &'cx Arc<BlockingTaskShared>,
    /// Required to make the context `!Send` and `!Sync`.
    _not_sync: PhantomData<*mut ()>,
}

impl<'cx> BlockingCtx<'cx> {
    /// Check whether the work should be aborted.
    ///
    /// Returns an instance of [`Aborted`] with an appropriate lifetime brand.
    pub fn check_aborted(self) -> Result<(), Aborted<'cx>> {
        if self.shared.should_abort() {
            Err(Aborted::new())
        } else {
            Ok(())
        }
    }
}

/// Signals that a task has been aborted.
#[derive(Debug, Clone, Copy)]
pub struct Aborted<'cx> {
    /// Lifetime brand.
    _ctx_brand: PhantomData<&'cx ()>,
}

impl<'cx> Aborted<'cx> {
    /// Create a new instance with any brand.
    ///
    /// This must not be public as we want to make sure that only appropriately-typed
    /// instances are created.
    fn new() -> Self {
        Self {
            _ctx_brand: PhantomData,
        }
    }
}

/// Create a context in which blocking work can be performed.
pub fn blocking<F, T: 'static + Send>(closure: F) -> BlockingTask<T>
where
    F: 'static + Send + for<'cx> FnOnce(BlockingCtx<'cx>) -> Result<T, Aborted<'cx>>,
{
    let shared = Arc::new(BlockingTaskShared::default());
    let handle = tokio::task::spawn_blocking({
        let shared = shared.clone();
        move || {
            closure(BlockingCtx {
                shared: &shared,
                _not_sync: PhantomData,
            })
            .map_err(|_| {
                // We need to erase the lifetime brand here as this result is sent back
                // to the calling context and hence must be `'static`.
                Aborted::new()
            })
        }
    });
    BlockingTask {
        handle: Some(handle),
        shared,
    }
}
