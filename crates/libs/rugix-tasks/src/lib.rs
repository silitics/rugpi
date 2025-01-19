//! Functionality for running cancelable blocking and asynchronous tasks.

use std::any::Any;
use std::fmt::{self, Debug};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use flume::Receiver;
use futures::FutureExt;
use pin_project::pin_project;
use scoped_tls::scoped_thread_local;
use tokio::runtime::{Handle, Runtime};
use tokio::sync::Notify;

scoped_thread_local! {
    /// Used to implicitly make [`TaskContext`] available to each task.
    static TASK_CONTEXT: TaskContext
}

/// Context in which a task is executed.
#[derive(Debug)]
struct TaskContext {
    /// Shared task state.
    shared_state: Arc<SharedTaskState>,
}

/// Used to make [`TaskContext`] available to a future.
#[pin_project]
pub struct TaskContextFuture<F> {
    context: TaskContext,
    #[pin]
    future: F,
}

impl<F: Future> Future for TaskContextFuture<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        TASK_CONTEXT.set(&this.context, || this.future.poll(cx))
    }
}

/// Used as a cancellation signal with [`std::panic::resume_unwind`].
#[derive(Debug, Clone, Copy)]
struct Canceled;

/// Check whether the task has been canceled and unwinds the stack if so.
#[inline]
pub fn check_canceled() {
    if !TASK_CONTEXT.is_set() {
        log_missing_context();
    } else if TASK_CONTEXT.with(|ctx| ctx.shared_state.should_abort()) {
        throw_canceled();
    }
}

/// Check whether the panic payload signals cancellation.
pub fn is_canceled_payload(payload: &Box<dyn Any + Send>) -> bool {
    payload.downcast_ref::<Canceled>().is_some()
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    check_canceled();
    match TOKIO_RUNTIME.handle.block_on(async move {
        tokio::select! { result = future => Some(result), _ = wait_for_cancellation() => None }
    }) {
        None => {
            throw_canceled();
            panic!("received canceled notification without unwinding?!?")
        }
        Some(value) => value,
    }
}

/// Unwind the stack with [`Canceled`].
#[inline(never)]
#[cold]
fn throw_canceled() -> () {
    // Canceling a future will unwind the stack. We use the same mechanism here to cancel
    // non-asynchronous tasks. Note that this requires unwinding support. If unwinding is
    // not supported, we will simply ignore the cancellation signal.
    #[cfg(panic = "unwind")]
    std::panic::resume_unwind(Box::new(Canceled));
    #[cfg(not(panic = "unwind"))]
    tracing::warn!("task has been canceled but will keep running")
}

/// Log an error in case the task context is missing.
#[inline(never)]
#[cold]
fn log_missing_context() -> () {
    tracing::error!("task context is required but not available");
}

/// Spawn an asynchronous task.
pub fn spawn<F>(future: F) -> Task<F::Output>
where
    F: 'static + Future + Send,
    F::Output: Send,
{
    let (task_msg_tx, task_msg_rx) = flume::bounded(1);
    let shared_state = Arc::new(SharedTaskState::default());
    let context = TaskContext {
        shared_state: shared_state.clone(),
    };
    let join_handle = JoinHandle::Tokio(TOKIO_RUNTIME.handle.spawn(async move {
        task_msg_tx
            .send(
                match AssertUnwindSafe(TaskContextFuture { future, context })
                    .catch_unwind()
                    .await
                {
                    Ok(output) => TaskStatusMsg::Finished { output },
                    Err(payload) => TaskStatusMsg::Panicked { payload },
                },
            )
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
    F: 'static + Send + FnOnce() -> T,
    T: 'static + Send,
{
    let (task_msg_tx, task_msg_rx) = flume::bounded(1);
    let shared_state = Arc::new(SharedTaskState::default());
    let context = TaskContext {
        shared_state: shared_state.clone(),
    };
    let join_handle = JoinHandle::Tokio(TOKIO_RUNTIME.handle.spawn_blocking(move || {
        task_msg_tx
            .send(
                match std::panic::catch_unwind(AssertUnwindSafe(move || {
                    TASK_CONTEXT.set(&context, closure)
                })) {
                    Ok(output) => TaskStatusMsg::Finished { output },
                    Err(payload) => TaskStatusMsg::Panicked { payload },
                },
            )
            .ok();
    }));
    Task {
        task_msg_rx,
        join_handle: Some(join_handle),
        shared_state,
        detached: false,
    }
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
pub fn shutdown_blocking() {
    let Some(runtime) = TOKIO_RUNTIME.runtime.lock().unwrap().take() else {
        panic!("runtime has already been taken")
    };
    runtime.shutdown_timeout(Duration::from_secs(10));
}

async fn wait_for_cancellation() {
    let shared_state = TASK_CONTEXT.with(|ctx| ctx.shared_state.clone());
    let notified = shared_state.notify_canceled.notified();
    if shared_state.should_abort() {
        return;
    }
    notified.await
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
    pub fn join_blocking(&self) -> T {
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
    notify_canceled: Notify,
}

impl SharedTaskState {
    /// Signals to the task that it should be aborted.
    pub fn signal_abort(&self) {
        self.should_abort.store(true, Ordering::Relaxed);
        #[cfg(panic = "unwind")]
        self.notify_canceled.notify_waiters();
    }

    /// Check whether the task should be aborted.
    pub fn should_abort(&self) -> bool {
        self.should_abort.load(Ordering::Relaxed)
    }
}
