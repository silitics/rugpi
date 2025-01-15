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
//! [`Aborted`] with the correct brand is to use [`BlockingCtx::check_abort`], which
//! checks whether the task has been aborted.
//!
//! In the future, we can also easily extend [`BlockingCtx`] to provide progress updates
//! for blocking tasks.
//!
//! Note that there would be a need for something like this even if Rugix was not using
//! any asynchronous code at all, as we would like to gracefully cancel operations (like
//! the creation of layers in Rugix Bakery) in any case. An alternative, maybe simpler
//! design would use a global signal to terminate all running tasks. This would, however,
//! not be as flexible.
//!
//! Signaling to a task that it should be aborted may not immediately abort the task. Just
//! like asynchronous code can only be aborted at `await` points, blocking code can only
//! be aborted at points where [`BlockingCtx::check_abort`] is called. Blocking code may
//! be in the midst of running an uninterruptible system call, and, even after initiating
//! an abort, a task may still need to perform some (blocking) cleanup work. Hence, it may
//! take some time for a task to finally abort.
//!
//! Checking wether a task should be aborted is a cheap operation, corresponding to an
//! atomic load and a branch. As a general rule of thumb, blocking code should ideally
//! check before any potentially blocking syscall whether the task should be aborted. This
//! ensures that a task is quickly aborted, even when carrying out multiple individually
//! fast operations in sequence.

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
#[must_use]
pub struct BlockingTask<T> {
    /// Handle obtained from [`tokio::task::spawn_blocking`].
    ///
    /// We use an [`Option`] here as we need to take out the handle in
    /// [`BlockingTask::abort`].
    handle: Option<tokio::task::JoinHandle<MaybeAborted<'static, T>>>,
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
            Ok(MaybeAborted::Done(value)) => Poll::Ready(value),
            Ok(MaybeAborted::Aborted(_)) => {
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

/// Blocking task that should be aborted.
pub struct AbortedTask<T> {
    /// Handle obtained from [`tokio::task::spawn_blocking`].
    handle: tokio::task::JoinHandle<MaybeAborted<'static, T>>,
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
            Ok(MaybeAborted::Done(value)) => Poll::Ready(Some(value)),
            Ok(MaybeAborted::Aborted(_)) => Poll::Ready(None),
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
    pub fn check_abort(self) -> MaybeAborted<'cx, ()> {
        if self.shared.should_abort() {
            MaybeAborted::Aborted(Aborted::new())
        } else {
            MaybeAborted::Done(())
        }
    }
}

/// Signals that a task has been aborted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Aborted<'cx> {
    /// Lifetime brand.
    _ctx_brand: PhantomData<&'cx ()>,
}

impl Aborted<'_> {
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

/// Result of an abortable operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[must_use]
pub enum MaybeAborted<'cx, T> {
    /// Operation has been aborted.
    Aborted(Aborted<'cx>),
    /// Operation has been completed.
    Done(T),
}

impl<'cx, T, E> MaybeAborted<'cx, Result<T, E>> {
    /// Transform the error of the result wrapped in [`MaybeAborted`].
    pub fn map_err<M, F>(self, map: M) -> MaybeAborted<'cx, Result<T, F>>
    where
        M: FnOnce(E) -> F,
    {
        match self {
            MaybeAborted::Aborted(aborted) => MaybeAborted::Aborted(aborted),
            MaybeAborted::Done(Ok(value)) => MaybeAborted::Done(Ok(value)),
            MaybeAborted::Done(Err(error)) => MaybeAborted::Done(Err(map(error))),
        }
    }
}

/// Create a blocking context and perform some work in it.
pub fn blocking<F, T: 'static + Send>(closure: F) -> BlockingTask<T>
where
    F: 'static + Send + for<'cx> FnOnce(BlockingCtx<'cx>) -> MaybeAborted<'cx, T>,
{
    let shared = Arc::new(BlockingTaskShared::default());
    let handle = tokio::task::spawn_blocking({
        let shared = shared.clone();
        move || {
            match closure(BlockingCtx {
                shared: &shared,
                _not_sync: PhantomData,
            }) {
                // We need to erase the lifetime brand here as this result is sent back
                // to the calling context and hence must be `'static`.
                MaybeAborted::Aborted(_) => MaybeAborted::Aborted(Aborted::new()),
                MaybeAborted::Done(value) => {
                    // Re-create with `'static` lifetime brand.
                    MaybeAborted::Done(value)
                }
            }
        }
    });
    BlockingTask {
        handle: Some(handle),
        shared,
    }
}

/// Return early if a task should be aborted.
///
/// This macro is used to check whether a blocking task should be aborted and propagate
/// the abort signal if necessary. It ensures that the task can be gracefully terminated
/// at specific points in the code.
///
/// # Examples
///
/// ```
/// # use rugix_blocking::{BlockingCtx, MaybeAborted, check_abort};
/// fn do_work<'cx>(cx: BlockingCtx<'cx>) -> MaybeAborted<'cx, ()> {
///     // Check if the task should be aborted and return early if so.
///     check_abort!(cx);
///     // Continue with the work if not aborted.
///     todo!("do some work")
/// }
/// ```
#[macro_export]
macro_rules! check_abort {
    ($cx:expr) => {
        if let $crate::MaybeAborted::Aborted(aborted) = $cx.check_abort() {
            return $crate::MaybeAborted::Aborted(aborted);
        }
    };
}

/// Perform a blocking operation and return early on abort signals (and errors).
///
/// This macro must be used in a function that returns [`MaybeAborted`].
///
/// This macro is used to handle the result of an abortable, blocking operation. If the
/// operation is aborted, the abort signal from the operation is propagated. Otherwise,
/// the value resulting from the operation is returned. In terms of types:
///
/// - `block!(MaybeAborted<'cx, T>) -> T`
///
/// If the operation is fallible, `block!(try ...)` can be used to also propagate errors:
///
/// - `block!(try MaybeAborted<'cx, Result<T, E>>) -> T`
/// - `block!(try Result<T, E>) -> T`
///
///
/// # Examples
///
/// ```
/// # use rugix_blocking::{BlockingCtx, MaybeAborted, block, check_abort};
/// fn do_work<'cx>(cx: BlockingCtx<'cx>) -> MaybeAborted<'cx, ()> {
///     check_abort!(cx);
///     // Propagate the abort signal if the inner operation is aborted.
///     let value = block!(do_work_inner(cx));
///     // Continue working with the returned value if not aborted.
///     todo!("`value` has type `u64`, continue the work with it")
/// }
///
/// fn do_work_inner<'cx>(cx: BlockingCtx<'cx>) -> MaybeAborted<'cx, u64> {
///     check_abort!(cx);
///     todo!("do some work, return `u64` when done")
/// }
/// ```
///
/// ```
/// # use rugix_blocking::{BlockingCtx, MaybeAborted, block, check_abort};
/// struct Error;
///
/// fn complex_operation<'cx>(cx: BlockingCtx<'cx>) -> MaybeAborted<'cx, Result<String, Error>> {
///     check_abort!(cx);
///     // Perform the first part of the operation and propagate any errors.
///     let part1 = block!(try first_part());
///     // Perform the second part of the operation with the result from the first part.
///     let part2 = block!(try second_part(cx, &part1));
///     // Combine the results and return.
///     MaybeAborted::Done(Ok(format!("{} + {}", part1, part2)))
/// }
///
/// fn first_part() -> Result<String, Error> {
///     todo!("do the first part of the work, return `String` on success")
/// }
///
/// fn second_part<'cx>(
///     cx: BlockingCtx<'cx>,
///     input: &str
/// ) -> MaybeAborted<'cx, Result<String, Error>>{
///     check_abort!(cx);
///     todo!("do the second part of the work with `input`, return `String` on success")
/// }
/// ```
#[macro_export]
macro_rules! block {
    (try $expr:expr) => {{
        match $crate::_private_block::_BlockingResult::branch($expr) {
            ::std::ops::ControlFlow::Break(result) => return result,
            ::std::ops::ControlFlow::Continue(value) => value,
        }
    }};
    ($expr:expr) => {{
        match $expr {
            $crate::MaybeAborted::Aborted(aborted) => {
                return $crate::MaybeAborted::Aborted(aborted)
            }
            $crate::MaybeAborted::Done(value) => value,
        }
    }};
}

#[doc(hidden)]
pub mod _private_block {
    use std::ops::ControlFlow;

    use crate::MaybeAborted;

    pub trait _BlockingResult<'cx, T, U, F> {
        fn branch(self) -> ControlFlow<MaybeAborted<'cx, Result<U, F>>, T>;
    }

    impl<'cx, T, E, U, F> _BlockingResult<'cx, T, U, F> for Result<T, E>
    where
        E: Into<F>,
    {
        fn branch(self) -> ControlFlow<MaybeAborted<'cx, Result<U, F>>, T> {
            match self {
                Ok(value) => ControlFlow::Continue(value),
                Err(error) => ControlFlow::Break(MaybeAborted::Done(Err(error.into()))),
            }
        }
    }

    impl<'cx, T, E, U, F> _BlockingResult<'cx, T, U, F> for MaybeAborted<'cx, Result<T, E>>
    where
        E: Into<F>,
    {
        fn branch(self) -> ControlFlow<MaybeAborted<'cx, Result<U, F>>, T> {
            match self {
                MaybeAborted::Aborted(aborted) => {
                    ControlFlow::Break(MaybeAborted::Aborted(aborted))
                }
                MaybeAborted::Done(Ok(value)) => ControlFlow::Continue(value),
                MaybeAborted::Done(Err(error)) => {
                    ControlFlow::Break(MaybeAborted::Done(Err(error.into())))
                }
            }
        }
    }
}
