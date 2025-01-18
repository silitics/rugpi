#![cfg_attr(feature = "nightly", feature(try_trait_v2))]
//! Functionality for running abortable tasks in asynchronous contexts.

use std::any::Any;
use std::convert::Infallible;
use std::fmt::{self, Debug};
use std::future::Future;
use std::ops::ControlFlow;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use flume::Receiver;
use futures::FutureExt;
use tokio::runtime::{Handle, Runtime};

/// Spawn an asynchronous task.
pub fn spawn<F>(future: F) -> Task<F::Output>
where
    F: 'static + Future + Send,
    F::Output: Send,
{
    let (task_msg_tx, task_msg_rx) = flume::bounded(1);
    let shared_state = Arc::new(SharedTaskState::default());
    let join_handle = JoinHandle::Tokio(TOKIO_RUNTIME.handle.spawn(async move {
        task_msg_tx
            .send(match AssertUnwindSafe(future).catch_unwind().await {
                Ok(output) => TaskStatusMsg::Finished { output },
                Err(payload) => TaskStatusMsg::Panicked { payload },
            })
            .ok();
    }));
    Task {
        task_msg_rx,
        join_handle: Some(join_handle),
        shared_state,
        detached: false,
    }
}

/// Spawn a blocking task.
pub fn spawn_blocking<F, T>(closure: F) -> Task<T>
where
    F: 'static + Send + FnOnce(BlockingCtx) -> T,
    T: 'static + Send,
{
    let (task_msg_tx, task_msg_rx) = flume::bounded(1);
    let shared_state = Arc::new(SharedTaskState::default());
    let join_handle = JoinHandle::Tokio(TOKIO_RUNTIME.handle.spawn_blocking({
        let shared_state = shared_state.clone();
        move || {
            // We ignore send errors here as we do not care about the receiver.
            let ctx = BlockingCtx {
                shared: &shared_state,
            };
            task_msg_tx
                .send(
                    match std::panic::catch_unwind(AssertUnwindSafe(move || closure(ctx))) {
                        Ok(output) => TaskStatusMsg::Finished { output },
                        Err(payload) => TaskStatusMsg::Panicked { payload },
                    },
                )
                .ok();
        }
    }));
    Task {
        task_msg_rx,
        join_handle: Some(join_handle),
        shared_state,
        detached: false,
    }
}

/// Spawn a blocking task.
pub fn spawn_blocking_abortable<F, T>(closure: F) -> Task<T>
where
    F: 'static + Send + FnOnce(BlockingCtx) -> MaybeAborted<T>,
    T: 'static + Send,
{
    let (task_msg_tx, task_msg_rx) = flume::bounded(1);
    let shared_state = Arc::new(SharedTaskState::default());
    let join_handle = JoinHandle::Tokio(TOKIO_RUNTIME.handle.spawn_blocking({
        let shared_state = shared_state.clone();
        move || {
            // We ignore send errors here as we do not care about the receiver.
            let ctx = BlockingCtx {
                shared: &shared_state,
            };
            task_msg_tx
                .send(
                    match std::panic::catch_unwind(AssertUnwindSafe(move || closure(ctx))) {
                        Ok(output) => match output {
                            MaybeAborted::Aborted => TaskStatusMsg::Aborted,
                            MaybeAborted::Done(output) => TaskStatusMsg::Finished { output },
                        },
                        Err(payload) => TaskStatusMsg::Panicked { payload },
                    },
                )
                .ok();
        }
    }));
    Task {
        task_msg_rx,
        join_handle: Some(join_handle),
        shared_state,
        detached: false,
    }
}

/// Run a blocking closure to completion in the current thread.
pub fn run_blocking_unchecked<F, T>(closure: F) -> T
where
    F: FnOnce(BlockingCtx) -> T,
{
    let shared_state = Arc::new(SharedTaskState::default());
    let ctx = BlockingCtx {
        shared: &shared_state,
    };
    closure(ctx)
}

/// Global Tokio runtime.
struct GlobalTokioRuntime {
    handle: Handle,
    runtime: Mutex<Option<Runtime>>,
}

/// Tokio runtime to execute asynchronous tasks.
static TOKIO_RUNTIME: LazyLock<GlobalTokioRuntime> = LazyLock::new(|| {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("unable to build tokio runtime");
    GlobalTokioRuntime {
        handle: runtime.handle().clone(),
        runtime: Mutex::new(Some(runtime)),
    }
});

/// Shutdown the Tokio runtime and abort all tasks.
pub fn shutdown_blocking(_: BlockingCtx) {
    let Some(runtime) = TOKIO_RUNTIME.runtime.lock().unwrap().take() else {
        panic!("runtime has already been taken")
    };
    runtime.shutdown_timeout(Duration::from_secs(10));
}

/// Owned handle to a task.
///
/// Dropping this handle will abort the task.
#[must_use]
pub struct Task<T> {
    // We use Flume here such that we can wait for messages from both blocking
    // and asynchronous contexts. Furthermore, this allows racing tasks against
    // each other using Flume's selector functionality.
    task_msg_rx: Receiver<TaskStatusMsg<T>>,
    join_handle: Option<JoinHandle>,
    shared_state: Arc<SharedTaskState>,
    detached: bool,
}

impl<T> Task<T> {
    /// Wait for the task to terminate and return it's result.
    pub async fn join(self) -> T {
        let Ok(msg) = self.task_msg_rx.recv_async().await else {
            panic!("task panicked without sending us a final message?!?")
        };
        self.handle_status_msg(msg)
    }

    /// Wait for the task to terminate and return it's result.
    pub fn join_blocking(&self, _: BlockingCtx<'_>) -> T {
        let Ok(msg) = self.task_msg_rx.recv() else {
            panic!("task panicked without sending us a final message?!?");
        };
        self.handle_status_msg(msg)
    }

    /// Handle the status message.
    fn handle_status_msg(&self, msg: TaskStatusMsg<T>) -> T {
        match msg {
            TaskStatusMsg::Finished { output } => output,
            TaskStatusMsg::Panicked { payload } => std::panic::resume_unwind(payload),
            TaskStatusMsg::Aborted => {
                panic!("task has been aborted without being dropped?!?")
            }
        }
    }

    /// Signal to the task that it should abort.
    fn signal_abort(&mut self) {
        self.shared_state.signal_abort();
        if let Some(JoinHandle::Tokio(handle)) = self.join_handle.take() {
            handle.abort();
        }
    }

    /// Detach the task.
    pub fn detach(mut self) {
        self.detached = true;
    }
}

impl<T> fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("task_id", &Arc::as_ptr(&self.shared_state))
            .field("should_abort", &self.shared_state.should_abort())
            .finish()
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if !self.detached {
            self.signal_abort()
        }
    }
}

/// Messages produced by tasks.
#[derive(Debug)]
enum TaskStatusMsg<T> {
    /// Task has finished successfully.
    Finished { output: T },
    /// Task has panicked.
    Panicked { payload: Box<dyn Any + Send> },
    /// Task has been aborted.
    Aborted,
}

/// Task join handle.
#[derive(Debug)]
enum JoinHandle {
    /// Join handle of a tokio task.
    Tokio(tokio::task::JoinHandle<()>),
    /// Join handle of a regular thread.
    #[expect(dead_code, reason = "spawning dedicated threads is not supported yet")]
    Thread(std::thread::JoinHandle<()>),
}

/// Shared task state.
#[derive(Debug, Default)]
struct SharedTaskState {
    /// Indicates wether the task should be aborted.
    should_abort: AtomicBool,
}

impl SharedTaskState {
    /// Signals to the task that it should be aborted.
    pub fn signal_abort(&self) {
        self.should_abort.store(true, Ordering::Relaxed);
    }

    /// Check whether the task should be aborted.
    pub fn should_abort(&self) -> bool {
        self.should_abort.load(Ordering::Relaxed)
    }
}

/// Context corresponding to a [`Task`] in which blocking work can be performed.
#[derive(Debug, Clone, Copy)]
pub struct BlockingCtx<'ctx> {
    /// Shared task data.
    shared: &'ctx Arc<SharedTaskState>,
}

impl BlockingCtx<'_> {
    /// Check whether the work should be aborted.
    pub fn should_abort(self) -> bool {
        self.shared.should_abort()
    }
}

/// Result of an abortable operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[must_use]
pub enum MaybeAborted<T> {
    /// Operation has been aborted.
    Aborted,
    /// Operation has been completed.
    Done(T),
}

impl<T> From<T> for MaybeAborted<T> {
    fn from(value: T) -> Self {
        MaybeAborted::Done(value)
    }
}

impl<T> MaybeAborted<T> {
    /// Unwrap the inner value.
    pub fn unwrap(self) -> T {
        match self {
            MaybeAborted::Aborted => {
                panic!("operation has been aborted, no value");
            }
            MaybeAborted::Done(value) => value,
        }
    }

    /// Transform the value wrapped in [`MaybeAborted`].
    pub fn map<M, U>(self, map: M) -> MaybeAborted<U>
    where
        M: FnOnce(T) -> U,
    {
        match self {
            MaybeAborted::Done(value) => MaybeAborted::Done(map(value)),
            MaybeAborted::Aborted => MaybeAborted::Aborted,
        }
    }
}

impl<T, E> MaybeAborted<Result<T, E>> {
    /// Transpose the [`MaybeAborted`] into the error.
    pub fn transpose_aborted(self) -> Result<T, MaybeAborted<E>> {
        match self {
            MaybeAborted::Aborted => Err(MaybeAborted::Aborted),
            MaybeAborted::Done(Ok(value)) => Ok(value),
            MaybeAborted::Done(Err(error)) => Err(MaybeAborted::Done(error)),
        }
    }

    /// Transform the value of the result wrapped in [`MaybeAborted`].
    pub fn map_ok<M, U>(self, map: M) -> MaybeAborted<Result<U, E>>
    where
        M: FnOnce(T) -> U,
    {
        match self {
            MaybeAborted::Aborted => MaybeAborted::Aborted,
            MaybeAborted::Done(Ok(value)) => MaybeAborted::Done(Ok(map(value))),
            MaybeAborted::Done(Err(error)) => MaybeAborted::Done(Err(error)),
        }
    }

    /// Transform the error of the result wrapped in [`MaybeAborted`].
    pub fn map_err<M, F>(self, map: M) -> MaybeAborted<Result<T, F>>
    where
        M: FnOnce(E) -> F,
    {
        match self {
            MaybeAborted::Aborted => MaybeAborted::Aborted,
            MaybeAborted::Done(Ok(value)) => MaybeAborted::Done(Ok(value)),
            MaybeAborted::Done(Err(error)) => MaybeAborted::Done(Err(map(error))),
        }
    }
}

impl<T> rugix_try::Try for MaybeAborted<T> {
    type Output = T;
    type Residual = MaybeAborted<Infallible>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Self::Done(output)
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Self::Aborted => ControlFlow::Break(MaybeAborted::Aborted),
            Self::Done(value) => ControlFlow::Continue(value),
        }
    }
}

impl<T> rugix_try::FromResidual<MaybeAborted<Infallible>> for MaybeAborted<T> {
    #[inline]
    fn from_residual(residual: MaybeAborted<Infallible>) -> Self {
        match residual {
            MaybeAborted::Aborted => MaybeAborted::Aborted,
        }
    }
}

impl<T, E, F> rugix_try::FromResidual<Result<Infallible, E>> for MaybeAborted<Result<T, F>>
where
    E: Into<F>,
{
    #[inline]
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Err(error) => MaybeAborted::Done(Err(error.into())),
        }
    }
}

impl<T> rugix_try::FromResidual<Option<Infallible>> for MaybeAborted<Option<T>> {
    #[inline]
    fn from_residual(residual: Option<Infallible>) -> Self {
        match residual {
            None => MaybeAborted::Done(None),
        }
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
        if $cx.should_abort() {
            return $crate::MaybeAborted::Aborted;
        }
    };
}
