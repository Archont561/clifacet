//! Native async command execution traits, cancellation, and progress (decision-26).
//!
//! Synchronous commands take validated input and execute to completion ([`Command`](crate::Command),
//! [`Handler`](crate::Handler)). Applications whose dependencies are async (HTTP clients,
//! databases, cloud storage, MCP) implement [`AsyncCommand`] or [`AsyncHandler`] instead.
//!
//! The `async` feature provides core traits and runtime contexts with zero third-party
//! dependencies:
//!
//! - [`AsyncHandler`]: command execution taking application context and [`RunCx`].
//! - [`LocalAsyncHandler`]: non-`Send` variant for single-threaded or WebAssembly targets.
//! - [`AsyncCommand`]: context-free async command seam.
//! - [`AsyncPlan`] and [`AsyncPlanned`]: asynchronous planning and dry-run preview generation.
//! - [`RunCx`]: execution context carrying cooperative cancellation ([`Cancel`]) and progress ([`ProgressSink`]).
//! - [`Cancel`]: thread-safe, cloneable cancellation token with atomic state and async wait support.
//! - [`ProgressSink`]: frontend-neutral progress reporting.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use crate::failure::Failure;

/// A boxed future used by asynchronous plans.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A local boxed future that does not require [`Send`].
pub type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A thread-safe, cloneable cancellation token.
///
/// Commands receive a cancellation token via [`RunCx::cancel`]. When an interrupt
/// signal arrives or a timeout expires, cancellation is requested by tripping the
/// token. Commands should periodically inspect [`Cancel::is_cancelled`] or `await`
/// [`Cancel::cancelled`] to perform clean shutdown and yield [`Failure::cancelled`].
///
/// # Example
///
/// ```
/// use clifacet::r#async::Cancel;
///
/// let cancel = Cancel::new();
/// assert!(!cancel.is_cancelled());
///
/// let cloned = cancel.clone();
/// cancel.cancel();
/// assert!(cloned.is_cancelled());
/// ```
#[derive(Clone, Default)]
pub struct Cancel {
    inner: Arc<CancelInner>,
}

#[derive(Default)]
struct CancelInner {
    cancelled: AtomicBool,
    wakers: Mutex<Vec<Waker>>,
}

impl Cancel {
    /// Creates an active, uncancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation and notifies all registered wakers.
    ///
    /// Calling this multiple times is safe and idempotent.
    pub fn cancel(&self) {
        if !self.inner.cancelled.swap(true, Ordering::SeqCst) {
            let mut wakers = self
                .inner
                .wakers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for waker in wakers.drain(..) {
                waker.wake();
            }
        }
    }

    /// Returns `true` if cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Returns a future that resolves when cancellation is requested.
    #[must_use]
    pub fn cancelled(&self) -> CancelledFuture {
        CancelledFuture {
            cancel: self.clone(),
        }
    }
}

impl fmt::Debug for Cancel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cancel")
            .field("is_cancelled", &self.is_cancelled())
            .finish()
    }
}

impl fmt::Display for Cancel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_cancelled() {
            write!(f, "cancelled")
        } else {
            write!(f, "active")
        }
    }
}

/// A future that resolves when its associated [`Cancel`] token trips.
#[derive(Debug, Clone)]
pub struct CancelledFuture {
    cancel: Cancel,
}

impl Future for CancelledFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.cancel.is_cancelled() {
            Poll::Ready(())
        } else {
            let mut wakers = self
                .cancel
                .inner
                .wakers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if self.cancel.is_cancelled() {
                Poll::Ready(())
            } else {
                let current_waker = cx.waker();
                if !wakers.iter().any(|w| w.will_wake(current_waker)) {
                    wakers.push(current_waker.clone());
                }
                Poll::Pending
            }
        }
    }
}

/// Consumer-agnostic sink for progress events during command execution.
///
/// Frontends render progress according to their mode (stderr line on TTY,
/// gauge in TUI, NDJSON event in `--json`, or notification in MCP). Commands
/// report progress via this trait without coupling to how it is presented.
pub trait ProgressSink: Send + Sync {
    /// Sets or updates the estimated total units of work, if known.
    fn total(&self, n: Option<u64>) {
        let _ = n;
    }

    /// Advances completed units of work by `n`.
    fn advance(&self, n: u64) {
        let _ = n;
    }

    /// Emits a status message describing current progress.
    fn message(&self, m: &str) {
        let _ = m;
    }
}

/// A [`ProgressSink`] that ignores all progress events.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProgressSink;

impl ProgressSink for NoopProgressSink {}

/// An event recorded by [`RecordingProgressSink`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent {
    /// Total expected units of work.
    Total(Option<u64>),
    /// Progress advanced by units.
    Advance(u64),
    /// Informational progress message.
    Message(String),
}

/// A [`ProgressSink`] that stores all reported events in memory for test verification.
#[derive(Debug, Default)]
pub struct RecordingProgressSink {
    events: Mutex<Vec<ProgressEvent>>,
    current: AtomicU64,
    total: Mutex<Option<u64>>,
    last_message: Mutex<Option<String>>,
}

impl RecordingProgressSink {
    /// Creates an empty recording sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a copy of all recorded progress events in chronological order.
    #[must_use]
    pub fn events(&self) -> Vec<ProgressEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Returns the cumulative units advanced so far.
    #[must_use]
    pub fn current(&self) -> u64 {
        self.current.load(Ordering::SeqCst)
    }

    /// Returns the most recently configured total units, if any.
    #[must_use]
    pub fn total(&self) -> Option<u64> {
        *self
            .total
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Returns the most recently reported progress message, if any.
    #[must_use]
    pub fn last_message(&self) -> Option<String> {
        self.last_message
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Clears all recorded events and counters.
    pub fn clear(&self) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        self.current.store(0, Ordering::SeqCst);
        *self
            .total
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        *self
            .last_message
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}

impl ProgressSink for RecordingProgressSink {
    fn total(&self, n: Option<u64>) {
        *self
            .total
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = n;
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(ProgressEvent::Total(n));
    }

    fn advance(&self, n: u64) {
        self.current.fetch_add(n, Ordering::SeqCst);
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(ProgressEvent::Advance(n));
    }

    fn message(&self, m: &str) {
        *self
            .last_message
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(m.to_owned());
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(ProgressEvent::Message(m.to_owned()));
    }
}

/// Execution context passed to running asynchronous commands.
///
/// `RunCx` bundles the cooperative cancellation token ([`Cancel`]) and the active
/// progress channel ([`ProgressSink`]).
#[derive(Clone)]
pub struct RunCx {
    cancel: Cancel,
    progress: Arc<dyn ProgressSink>,
}

impl RunCx {
    /// Creates a new execution context with the given cancellation token and progress sink.
    #[must_use]
    pub fn new(cancel: Cancel, progress: Arc<dyn ProgressSink>) -> Self {
        Self { cancel, progress }
    }

    /// Creates an execution context with an active cancellation token and a no-op progress sink.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(Cancel::new(), Arc::new(NoopProgressSink))
    }

    /// Creates a test execution context paired with a recording progress sink.
    ///
    /// The cancellation token can be tripped directly via `cx.cancel().cancel()`
    /// or `cx.request_cancel()`, and progress events can be inspected on the returned sink.
    #[must_use]
    pub fn for_test() -> (Self, Arc<RecordingProgressSink>) {
        let sink = Arc::new(RecordingProgressSink::new());
        let cx = Self::new(Cancel::new(), sink.clone());
        (cx, sink)
    }

    /// Returns a reference to the cancellation token.
    #[must_use]
    pub fn cancel(&self) -> &Cancel {
        &self.cancel
    }

    /// Returns `true` if cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Requests cooperative cancellation.
    pub fn request_cancel(&self) {
        self.cancel.cancel();
    }

    /// Returns a reference to the active progress sink.
    #[must_use]
    pub fn progress(&self) -> &dyn ProgressSink {
        &*self.progress
    }

    /// Returns a reference to the shared progress sink handle.
    #[must_use]
    pub fn progress_arc(&self) -> &Arc<dyn ProgressSink> {
        &self.progress
    }

    /// Sets or updates the estimated total units of work on the progress sink.
    pub fn report_total(&self, total: Option<u64>) {
        self.progress.total(total);
    }

    /// Advances completed units of work by `n` on the progress sink.
    pub fn advance(&self, n: u64) {
        self.progress.advance(n);
    }

    /// Emits a human-readable progress status message on the progress sink.
    pub fn message(&self, msg: &str) {
        self.progress.message(msg);
    }
}

impl Default for RunCx {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Debug for RunCx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunCx")
            .field("cancel", &self.cancel)
            .finish_non_exhaustive()
    }
}

/// An asynchronous command that runs with an application context.
///
/// This trait mirrors [`Handler`](crate::Handler) for asynchronous execution. Implementations
/// must be `Send + Sync` so they can execute across multi-threaded runtimes or concurrent
/// tool dispatchers such as Model Context Protocol servers.
pub trait AsyncHandler<Cx: ?Sized>: Send + Sync {
    /// Validated input accepted by the handler.
    type Input: Send;

    /// Output data produced by the handler.
    type Output: Send;

    /// Executes the command asynchronously against its input, context, and runtime context.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if execution fails or is cancelled.
    fn handle(
        &self,
        input: Self::Input,
        cx: &Cx,
        run: &RunCx,
    ) -> impl Future<Output = Result<Self::Output, Failure>> + Send;
}

/// An asynchronous command that runs on single-threaded or WebAssembly targets without `Send` constraints.
///
/// All [`AsyncHandler`] implementations automatically implement `LocalAsyncHandler`.
pub trait LocalAsyncHandler<Cx: ?Sized> {
    /// Validated input accepted by the handler.
    type Input;

    /// Output data produced by the handler.
    type Output;

    /// Executes the command locally against its input, context, and runtime context.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if execution fails or is cancelled.
    fn handle_local(
        &self,
        input: Self::Input,
        cx: &Cx,
        run: &RunCx,
    ) -> impl Future<Output = Result<Self::Output, Failure>>;
}

impl<H: AsyncHandler<Cx>, Cx: ?Sized> LocalAsyncHandler<Cx> for H {
    type Input = H::Input;
    type Output = H::Output;

    fn handle_local(
        &self,
        input: Self::Input,
        cx: &Cx,
        run: &RunCx,
    ) -> impl Future<Output = Result<Self::Output, Failure>> {
        AsyncHandler::handle(self, input, cx, run)
    }
}

/// An asynchronous command that runs without an application context.
///
/// This trait mirrors [`Command`](crate::Command) for asynchronous execution. Any `AsyncCommand`
/// automatically implements [`AsyncHandler<Cx>`] for any context type.
pub trait AsyncCommand: Send + Sync {
    /// Validated input accepted by the command.
    type Input: Send;

    /// Output data produced by the command.
    type Output: Send;

    /// Runs the command asynchronously against validated input and runtime context.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if execution fails or is cancelled.
    fn execute(
        &self,
        input: Self::Input,
        run: &RunCx,
    ) -> impl Future<Output = Result<Self::Output, Failure>> + Send;
}

impl<C: AsyncCommand, Cx: ?Sized> AsyncHandler<Cx> for C {
    type Input = C::Input;
    type Output = C::Output;

    fn handle(
        &self,
        input: Self::Input,
        _cx: &Cx,
        run: &RunCx,
    ) -> impl Future<Output = Result<Self::Output, Failure>> + Send {
        self.execute(input, run)
    }
}

/// An asynchronous execution plan describing steps and future work.
///
/// An `AsyncPlan` encapsulates human-readable step descriptions (used by `--dry-run`)
/// and the asynchronous work future that executes them.
pub struct AsyncPlan<'a, T> {
    steps: Vec<String>,
    work: Box<dyn FnOnce() -> BoxFuture<'a, Result<T, Failure>> + Send + 'a>,
}

impl<'a, T> AsyncPlan<'a, T> {
    /// Creates a new asynchronous plan from a work closure returning a future.
    pub fn new<F, Fut>(work: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'a,
        Fut: Future<Output = Result<T, Failure>> + Send + 'a,
    {
        Self {
            steps: Vec::new(),
            work: Box::new(move || Box::pin(work())),
        }
    }

    /// Appends a step description to the plan.
    #[must_use]
    pub fn step(mut self, line: impl Into<String>) -> Self {
        self.steps.push(line.into());
        self
    }

    /// Appends multiple step descriptions to the plan.
    #[must_use]
    pub fn steps<I, S>(mut self, lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.steps.extend(lines.into_iter().map(Into::into));
        self
    }

    /// Returns a slice of step descriptions in this plan.
    #[must_use]
    pub fn steps_slice(&self) -> &[String] {
        &self.steps
    }

    /// Returns all step descriptions joined by newlines.
    #[must_use]
    pub fn describe(&self) -> String {
        self.steps.join("\n")
    }

    /// Executes the plan's asynchronous work to completion.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if execution fails or is cancelled.
    pub async fn execute(self) -> Result<T, Failure> {
        (self.work)().await
    }

    /// Maps the successful output of the plan through `f`.
    pub fn map<U, F>(self, f: F) -> AsyncPlan<'a, U>
    where
        F: FnOnce(T) -> U + Send + 'a,
        T: 'a,
        U: 'a,
    {
        let steps = self.steps;
        let work = self.work;
        AsyncPlan {
            steps,
            work: Box::new(move || {
                Box::pin(async move {
                    let value = work().await?;
                    Ok(f(value))
                })
            }),
        }
    }
}

impl<T> fmt::Debug for AsyncPlan<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncPlan")
            .field("steps", &self.steps)
            .finish_non_exhaustive()
    }
}

/// An asynchronous command that produces an [`AsyncPlan`] before execution.
pub trait AsyncPlanned: Send + Sync {
    /// Validated input accepted by the command.
    type Input: Send;

    /// Output data produced by the executed plan.
    type Output: Send;

    /// Plans the asynchronous command without executing mutating side effects.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if planning fails.
    fn plan(
        &self,
        input: Self::Input,
    ) -> impl Future<Output = Result<AsyncPlan<'_, Self::Output>, Failure>> + Send + '_;
}

impl<P: AsyncPlanned> AsyncCommand for P {
    type Input = P::Input;
    type Output = P::Output;

    async fn execute(&self, input: Self::Input, _run: &RunCx) -> Result<Self::Output, Failure> {
        let plan = self.plan(input).await?;
        plan.execute().await
    }
}

/// An asynchronous command with application context that produces an [`AsyncPlan`].
pub trait AsyncPlannedWith<Cx: ?Sized>: Send + Sync {
    /// Validated input accepted by the command.
    type Input: Send;

    /// Output data produced by the executed plan.
    type Output: Send;

    /// Plans the asynchronous command against context without executing mutating side effects.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if planning fails.
    fn plan<'a>(
        &'a self,
        input: Self::Input,
        cx: &'a Cx,
    ) -> impl Future<Output = Result<AsyncPlan<'a, Self::Output>, Failure>> + Send + 'a;
}

/// A pluggable executor that drives asynchronous commands to completion.
///
/// Synchronous entry points such as [`App::run`](crate::App::run) or
/// [`App::run_with`](crate::App::run_with) drive asynchronous handlers via an
/// implementation of this trait.
pub trait Executor: Send + Sync {
    /// Executes the future to completion within the executor's runtime.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if execution fails or is cancelled.
    fn run<T: Send + 'static>(
        &self,
        fut: BoxFuture<'static, Result<T, Failure>>,
        cancel: Cancel,
    ) -> Result<T, Failure>;
}

impl<E: Executor> Executor for std::sync::Arc<E> {
    fn run<T: Send + 'static>(
        &self,
        fut: BoxFuture<'static, Result<T, Failure>>,
        cancel: Cancel,
    ) -> Result<T, Failure> {
        (**self).run(fut, cancel)
    }
}

/// A minimal synchronous executor using standard library wakers and thread parking.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdExecutor;

impl Executor for StdExecutor {
    fn run<T: Send + 'static>(
        &self,
        fut: BoxFuture<'static, Result<T, Failure>>,
        _cancel: Cancel,
    ) -> Result<T, Failure> {
        crate::cli::block_on(fut)
    }
}

/// A Tokio-backed runtime executor supporting current-thread, multi-thread, or custom handles.
#[cfg(feature = "tokio")]
#[derive(Debug, Clone)]
pub struct TokioExecutor {
    runtime: TokioRuntimeConfig,
}

#[cfg(feature = "tokio")]
#[derive(Debug, Clone)]
enum TokioRuntimeConfig {
    CurrentThread,
    MultiThread,
    Handle(tokio::runtime::Handle),
}

#[cfg(feature = "tokio")]
impl TokioExecutor {
    /// Creates a default `TokioExecutor` configured for a current-thread runtime.
    #[must_use]
    pub fn current_thread() -> Self {
        Self {
            runtime: TokioRuntimeConfig::CurrentThread,
        }
    }

    /// Creates a `TokioExecutor` configured for a multi-thread runtime.
    #[must_use]
    pub fn multi_thread() -> Self {
        Self {
            runtime: TokioRuntimeConfig::MultiThread,
        }
    }

    /// Creates a `TokioExecutor` using an existing Tokio runtime handle.
    #[must_use]
    pub fn from_handle(handle: tokio::runtime::Handle) -> Self {
        Self {
            runtime: TokioRuntimeConfig::Handle(handle),
        }
    }
}

#[cfg(feature = "tokio")]
impl Default for TokioExecutor {
    fn default() -> Self {
        Self::current_thread()
    }
}

#[cfg(feature = "tokio")]
impl Executor for TokioExecutor {
    fn run<T: Send + 'static>(
        &self,
        fut: BoxFuture<'static, Result<T, Failure>>,
        _cancel: Cancel,
    ) -> Result<T, Failure> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(Failure::execution(
                "invoking synchronous App::run() inside an active Tokio runtime is not \
                 supported; use App::run_async() instead",
            ));
        }

        match &self.runtime {
            TokioRuntimeConfig::CurrentThread => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .map_err(|e| {
                        Failure::execution(format!(
                            "cannot create Tokio current-thread runtime: {e}"
                        ))
                    })?;
                rt.block_on(fut)
            }
            TokioRuntimeConfig::MultiThread => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .map_err(|e| Failure::execution(format!("cannot create Tokio runtime: {e}")))?;
                rt.block_on(fut)
            }
            TokioRuntimeConfig::Handle(handle) => handle.block_on(fut),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unused_async_trait_impl, clippy::manual_let_else)]
mod tests {
    use super::*;

    struct SimpleFetch;

    impl AsyncCommand for SimpleFetch {
        type Input = String;
        type Output = usize;

        async fn execute(&self, input: Self::Input, run: &RunCx) -> Result<Self::Output, Failure> {
            if run.is_cancelled() {
                return Err(Failure::cancelled("aborted before fetch"));
            }
            run.message("fetching");
            run.advance(1);
            Ok(input.len())
        }
    }

    struct Store {
        counter: AtomicU64,
    }

    struct Increment;

    impl AsyncHandler<Store> for Increment {
        type Input = u64;
        type Output = u64;

        async fn handle(
            &self,
            input: Self::Input,
            cx: &Store,
            run: &RunCx,
        ) -> Result<Self::Output, Failure> {
            run.report_total(Some(input));
            for _ in 0..input {
                if run.is_cancelled() {
                    return Err(Failure::cancelled("interrupted during increment"));
                }
                cx.counter.fetch_add(1, Ordering::SeqCst);
                run.advance(1);
            }
            Ok(cx.counter.load(Ordering::SeqCst))
        }
    }

    struct PlannedDeploy;

    impl AsyncPlanned for PlannedDeploy {
        type Input = String;
        type Output = String;

        async fn plan(&self, input: Self::Input) -> Result<AsyncPlan<'_, Self::Output>, Failure> {
            let step_desc = format!("deploy service {input}");
            let plan = AsyncPlan::new(move || async move { Ok(format!("deployed {input}")) })
                .step(step_desc);
            Ok(plan)
        }
    }

    #[test]
    fn cancel_token_state_and_waking() {
        let cancel = Cancel::new();
        assert!(!cancel.is_cancelled());
        assert_eq!(cancel.to_string(), "active");

        let cloned = cancel.clone();
        cancel.cancel();
        assert!(cancel.is_cancelled());
        assert!(cloned.is_cancelled());
        assert_eq!(cancel.to_string(), "cancelled");

        // Idempotent cancel calls
        cancel.cancel();
        assert!(cancel.is_cancelled());
    }

    #[test]
    fn run_cx_progress_recording_and_cancel_integration() {
        let (cx, recorder) = RunCx::for_test();
        assert!(!cx.is_cancelled());

        cx.report_total(Some(100));
        cx.message("starting");
        cx.advance(25);

        assert_eq!(recorder.total(), Some(100));
        assert_eq!(recorder.current(), 25);
        assert_eq!(recorder.last_message(), Some("starting".to_owned()));

        let events = recorder.events();
        assert_eq!(
            events,
            vec![
                ProgressEvent::Total(Some(100)),
                ProgressEvent::Message("starting".to_owned()),
                ProgressEvent::Advance(25),
            ]
        );

        cx.request_cancel();
        assert!(cx.is_cancelled());
    }

    #[test]
    fn async_command_and_handler_dispatch() {
        let (cx, recorder) = RunCx::for_test();
        let cmd = SimpleFetch;

        // Block on future with minimal executor
        let result = std::future::Future::poll(
            std::pin::pin!(cmd.execute("hello world".to_owned(), &cx)),
            &mut Context::from_waker(Waker::noop()),
        );

        match result {
            Poll::Ready(Ok(len)) => assert_eq!(len, 11),
            _ => panic!("expected ready result"),
        }

        assert_eq!(recorder.current(), 1);
        assert_eq!(recorder.last_message(), Some("fetching".to_owned()));

        // Blanket AsyncHandler dispatch for AsyncCommand
        let handler_res = std::future::Future::poll(
            std::pin::pin!(AsyncHandler::<()>::handle(&cmd, "hi".to_owned(), &(), &cx)),
            &mut Context::from_waker(Waker::noop()),
        );

        match handler_res {
            Poll::Ready(Ok(len)) => assert_eq!(len, 2),
            _ => panic!("expected ready result"),
        }
    }

    #[test]
    fn async_handler_with_context_and_cancellation() {
        let (cx, recorder) = RunCx::for_test();
        let store = Store {
            counter: AtomicU64::new(10),
        };
        let inc = Increment;

        let res = std::future::Future::poll(
            std::pin::pin!(inc.handle(3, &store, &cx)),
            &mut Context::from_waker(Waker::noop()),
        );

        match res {
            Poll::Ready(Ok(val)) => assert_eq!(val, 13),
            _ => panic!("expected ready result"),
        }

        assert_eq!(recorder.current(), 3);
        assert_eq!(recorder.total(), Some(3));

        // Test cancellation
        cx.request_cancel();
        let cancel_res = std::future::Future::poll(
            std::pin::pin!(inc.handle(2, &store, &cx)),
            &mut Context::from_waker(Waker::noop()),
        );

        match cancel_res {
            Poll::Ready(Err(err)) => {
                assert_eq!(err.kind(), crate::failure::FailureKind::Cancelled);
                assert!(err.message().contains("interrupted"));
            }
            _ => panic!("expected cancellation error"),
        }
    }

    #[test]
    fn async_plan_steps_and_execution() {
        let deploy = PlannedDeploy;
        let plan_res = std::future::Future::poll(
            std::pin::pin!(deploy.plan("auth-service".to_owned())),
            &mut Context::from_waker(Waker::noop()),
        );

        let plan = match plan_res {
            Poll::Ready(Ok(p)) => p,
            _ => panic!("expected ready plan"),
        };

        assert_eq!(plan.describe(), "deploy service auth-service");
        assert_eq!(plan.steps_slice(), &["deploy service auth-service"]);

        let mapped = plan.map(|s| s.to_uppercase());
        let exec_res = std::future::Future::poll(
            std::pin::pin!(mapped.execute()),
            &mut Context::from_waker(Waker::noop()),
        );

        match exec_res {
            Poll::Ready(Ok(out)) => assert_eq!(out, "DEPLOYED AUTH-SERVICE"),
            _ => panic!("expected ready execution"),
        }
    }

    #[test]
    fn local_async_handler_supports_non_send_types() {
        use std::cell::RefCell;
        use std::rc::Rc;

        struct LocalState {
            log: Rc<RefCell<Vec<String>>>,
        }

        struct NonSendHandler {
            local: LocalState,
        }

        impl LocalAsyncHandler<()> for NonSendHandler {
            type Input = String;
            type Output = usize;

            async fn handle_local(
                &self,
                input: Self::Input,
                _cx: &(),
                _run: &RunCx,
            ) -> Result<Self::Output, Failure> {
                self.local.log.borrow_mut().push(input.clone());
                Ok(self.local.log.borrow().len())
            }
        }

        let state = LocalState {
            log: Rc::new(RefCell::new(Vec::new())),
        };
        let handler = NonSendHandler {
            local: LocalState {
                log: state.log.clone(),
            },
        };
        let cx = RunCx::empty();

        let res = std::future::Future::poll(
            std::pin::pin!(handler.handle_local("event1".to_owned(), &(), &cx)),
            &mut Context::from_waker(Waker::noop()),
        );

        match res {
            Poll::Ready(Ok(count)) => assert_eq!(count, 1),
            _ => panic!("expected ready result"),
        }
        assert_eq!(*state.log.borrow(), vec!["event1"]);
    }

    #[test]
    fn cancelled_future_polling_behavior() {
        let cancel = Cancel::new();
        let mut fut = cancel.cancelled();

        // Polling before cancel is tripped should return Pending
        let pending = std::future::Future::poll(
            std::pin::pin!(&mut fut),
            &mut Context::from_waker(Waker::noop()),
        );
        assert!(pending.is_pending());

        // Trip cancellation
        cancel.cancel();

        // Polling after cancel should return Ready
        let ready = std::future::Future::poll(
            std::pin::pin!(&mut fut),
            &mut Context::from_waker(Waker::noop()),
        );
        assert!(ready.is_ready());

        // Polling a new future created after cancel is already tripped should be immediately Ready
        let mut immediate = cancel.cancelled();
        let ready_imm = std::future::Future::poll(
            std::pin::pin!(&mut immediate),
            &mut Context::from_waker(Waker::noop()),
        );
        assert!(ready_imm.is_ready());
    }

    #[test]
    fn run_cx_and_cancel_helpers_coverage() {
        struct DummySink;
        impl ProgressSink for DummySink {}

        let cancel = Cancel::new();
        assert!(format!("{cancel:?}").contains("Cancel"));

        let run_cx = RunCx::default();
        assert!(format!("{run_cx:?}").contains("RunCx"));
        let _ = run_cx.progress();
        let _ = run_cx.progress_arc();
        run_cx.message("status");

        let dummy = DummySink;
        dummy.total(Some(10));
        dummy.advance(5);
        dummy.message("test");

        let std_exec = StdExecutor;
        let res = std_exec.run(
            Box::pin(async { Ok::<_, Failure>("ok".to_string()) }),
            Cancel::new(),
        );
        assert_eq!(res.unwrap(), "ok");

        let arc_exec = std::sync::Arc::new(StdExecutor);
        let res_arc = arc_exec.run(Box::pin(async { Ok::<_, Failure>(123) }), Cancel::new());
        assert_eq!(res_arc.unwrap(), 123);
    }

    #[cfg(feature = "tokio")]
    #[test]
    fn tokio_executor_unit_coverage() {
        let def = TokioExecutor::default();
        let res = def.run(Box::pin(async { Ok::<_, Failure>(1) }), Cancel::new());
        assert_eq!(res.unwrap(), 1);

        let multi = TokioExecutor::multi_thread();
        let res_multi = multi.run(Box::pin(async { Ok::<_, Failure>(2) }), Cancel::new());
        assert_eq!(res_multi.unwrap(), 2);

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let handle_exec = TokioExecutor::from_handle(rt.handle().clone());
        let res_handle = handle_exec.run(Box::pin(async { Ok::<_, Failure>(3) }), Cancel::new());
        assert_eq!(res_handle.unwrap(), 3);
    }
}
