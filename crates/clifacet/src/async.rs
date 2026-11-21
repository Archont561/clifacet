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
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
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
    interrupt_count: AtomicUsize,
    wakers: Mutex<Vec<Waker>>,
    abort_wakers: Mutex<Vec<Waker>>,
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

    /// Signals an interrupt (Ctrl-C) event.
    ///
    /// The first invocation trips the cancellation token (requesting cooperative cancellation).
    /// The second (or subsequent) invocation requests immediate abort, waking tasks
    /// awaiting [`aborted`](Self::aborted).
    pub fn interrupt(&self) {
        let prev = self.inner.interrupt_count.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            self.cancel();
        } else {
            let mut wakers = self
                .inner
                .abort_wakers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for waker in wakers.drain(..) {
                waker.wake();
            }
        }
    }

    /// Requests an immediate hard abort, waking both cancellation and abort listeners.
    pub fn abort(&self) {
        self.cancel();
        self.inner.interrupt_count.store(2, Ordering::SeqCst);
        let mut wakers = self
            .inner
            .abort_wakers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for waker in wakers.drain(..) {
            waker.wake();
        }
    }

    /// Returns `true` if cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Returns `true` if an immediate hard abort has been requested.
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        self.inner.interrupt_count.load(Ordering::SeqCst) >= 2
    }

    /// Returns the number of registered interrupt signals.
    #[must_use]
    pub fn interrupt_count(&self) -> usize {
        self.inner.interrupt_count.load(Ordering::SeqCst)
    }

    /// Returns a future that resolves when cancellation is requested.
    #[must_use]
    pub fn cancelled(&self) -> CancelledFuture {
        CancelledFuture {
            cancel: self.clone(),
        }
    }

    /// Returns a future that resolves when an immediate hard abort is requested.
    #[must_use]
    pub fn aborted(&self) -> AbortedFuture {
        AbortedFuture {
            cancel: self.clone(),
        }
    }
}

impl fmt::Debug for Cancel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cancel")
            .field("is_cancelled", &self.is_cancelled())
            .field("is_aborted", &self.is_aborted())
            .field("interrupt_count", &self.interrupt_count())
            .finish()
    }
}

impl fmt::Display for Cancel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_aborted() {
            write!(f, "aborted")
        } else if self.is_cancelled() {
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

/// A future that resolves when its associated [`Cancel`] token signals hard abort.
#[derive(Debug, Clone)]
pub struct AbortedFuture {
    cancel: Cancel,
}

impl Future for AbortedFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.cancel.is_aborted() {
            Poll::Ready(())
        } else {
            let mut wakers = self
                .cancel
                .inner
                .abort_wakers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if self.cancel.is_aborted() {
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

    /// Returns `true` if an immediate hard abort has been requested.
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        self.cancel.is_aborted()
    }

    /// Requests cooperative cancellation.
    pub fn request_cancel(&self) {
        self.cancel.cancel();
    }

    /// Signals an interrupt (e.g. Ctrl-C), requesting cancellation on first call and abort on second.
    pub fn request_interrupt(&self) {
        self.cancel.interrupt();
    }

    /// Requests an immediate hard abort.
    pub fn request_abort(&self) {
        self.cancel.abort();
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

    /// What the command would do with `input` and context without executing mutating side effects.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::validation`] by default when the command cannot be previewed.
    fn preview<'a>(
        &'a self,
        input: Self::Input,
        cx: &'a Cx,
    ) -> impl Future<Output = Result<AsyncPlan<'a, Self::Output>, Failure>> + Send + 'a {
        let _ = (input, cx);
        async {
            Err(Failure::validation(
                "this command cannot describe what it would do, so it cannot be run with --dry-run",
            ))
        }
    }
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

    /// What the command would do with `input` and context without executing mutating side effects.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::validation`] by default when the command cannot be previewed.
    fn preview_local<'a>(
        &'a self,
        input: Self::Input,
        cx: &'a Cx,
    ) -> impl Future<Output = Result<AsyncPlan<'a, Self::Output>, Failure>> + 'a {
        let _ = (input, cx);
        async {
            Err(Failure::validation(
                "this command cannot describe what it would do, so it cannot be run with --dry-run",
            ))
        }
    }
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

    fn preview_local<'a>(
        &'a self,
        input: Self::Input,
        cx: &'a Cx,
    ) -> impl Future<Output = Result<AsyncPlan<'a, Self::Output>, Failure>> + 'a {
        AsyncHandler::preview(self, input, cx)
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

    /// Generates an asynchronous plan describing the steps of this command without executing mutating work.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::validation`] by default when the command cannot be dry-run.
    fn preview(
        &self,
        input: Self::Input,
    ) -> impl Future<Output = Result<AsyncPlan<'_, Self::Output>, Failure>> + Send + '_ {
        let _ = input;
        async {
            Err(Failure::validation(
                "this command cannot describe what it would do, so it cannot be run with --dry-run",
            ))
        }
    }
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

    fn preview<'a>(
        &'a self,
        input: Self::Input,
        _cx: &'a Cx,
    ) -> impl Future<Output = Result<AsyncPlan<'a, Self::Output>, Failure>> + Send + 'a {
        self.preview(input)
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

    /// Returns `true` if this plan contains no steps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Returns the number of steps in this plan.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
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

    async fn preview(&self, input: Self::Input) -> Result<AsyncPlan<'_, Self::Output>, Failure> {
        self.plan(input).await
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
    grace_period: std::time::Duration,
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
            grace_period: std::time::Duration::from_secs(5),
        }
    }

    /// Creates a `TokioExecutor` configured for a multi-thread runtime.
    #[must_use]
    pub fn multi_thread() -> Self {
        Self {
            runtime: TokioRuntimeConfig::MultiThread,
            grace_period: std::time::Duration::from_secs(5),
        }
    }

    /// Creates a `TokioExecutor` using an existing Tokio runtime handle.
    #[must_use]
    pub fn from_handle(handle: tokio::runtime::Handle) -> Self {
        Self {
            runtime: TokioRuntimeConfig::Handle(handle),
            grace_period: std::time::Duration::from_secs(5),
        }
    }

    /// Sets the grace period to wait between first cancellation signal and hard abort (default 5s).
    #[must_use]
    pub fn grace_period(mut self, period: std::time::Duration) -> Self {
        self.grace_period = period;
        self
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
        cancel: Cancel,
    ) -> Result<T, Failure> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(Failure::execution(
                "invoking synchronous App::run() inside an active Tokio runtime is not \
                 supported; use App::run_async() instead",
            ));
        }

        let cancellable = run_with_cancellation(fut, cancel, self.grace_period, None);

        match &self.runtime {
            TokioRuntimeConfig::CurrentThread => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        Failure::execution(format!(
                            "cannot create Tokio current-thread runtime: {e}"
                        ))
                    })?;
                rt.block_on(cancellable)
            }
            TokioRuntimeConfig::MultiThread => {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| Failure::execution(format!("cannot create Tokio runtime: {e}")))?;
                rt.block_on(cancellable)
            }
            TokioRuntimeConfig::Handle(handle) => handle.block_on(cancellable),
        }
    }
}

#[cfg(feature = "tokio")]
struct SafeFuture<F> {
    inner: Option<Pin<Box<F>>>,
}

#[cfg(feature = "tokio")]
impl<F: Future> SafeFuture<F> {
    fn new(fut: F) -> Self {
        Self {
            inner: Some(Box::pin(fut)),
        }
    }
}

#[cfg(feature = "tokio")]
impl<F: Future> Future for SafeFuture<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(inner) = self.inner.as_mut() else {
            return Poll::Pending;
        };
        let waker = cx.waker().clone();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut cx2 = Context::from_waker(&waker);
            inner.as_mut().poll(&mut cx2)
        }));
        if let Ok(poll) = res {
            poll
        } else {
            self.inner = None;
            Poll::Pending
        }
    }
}

/// Runs `fut` under cooperative cancellation, grace period, and optional timeout.
///
/// 1. A first Ctrl-C signal (or external cancellation request) trips the [`Cancel`] token.
///    The future is awaited during the `grace_period` to allow cooperative cleanup.
/// 2. A second Ctrl-C signal (or expiration of the `grace_period`) immediately aborts
///    the future and returns `Err(Failure::cancelled("operation aborted by interrupt"))`.
/// 3. If `timeout` is specified and expires before completion, the future is immediately
///    aborted and returns `Err(Failure::execution(format!("timed out after ...")))`.
///
/// # Errors
///
/// Returns the future's failure, or an execution/cancelled failure on timeout or interrupt abort.
#[cfg(feature = "tokio")]
#[allow(clippy::too_many_lines)]
pub async fn run_with_cancellation<T>(
    fut: impl Future<Output = Result<T, Failure>>,
    cancel: Cancel,
    grace_period: std::time::Duration,
    timeout: Option<std::time::Duration>,
) -> Result<T, Failure> {
    if cancel.is_aborted() {
        return Err(Failure::cancelled("operation aborted by interrupt"));
    }

    let cancel_for_signal = cancel.clone();
    let signal_listener = async move {
        let first_signal = SafeFuture::new(async {
            let ctrl_c =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(tokio::signal::ctrl_c));
            if let Ok(fut) = ctrl_c {
                SafeFuture::new(fut).await
            } else {
                std::future::pending::<std::io::Result<()>>().await
            }
        });
        if first_signal.await.is_ok() {
            cancel_for_signal.interrupt();
            let second_signal = SafeFuture::new(async {
                let ctrl_c =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(tokio::signal::ctrl_c));
                if let Ok(fut) = ctrl_c {
                    SafeFuture::new(fut).await
                } else {
                    std::future::pending::<std::io::Result<()>>().await
                }
            });
            if second_signal.await.is_ok() {
                cancel_for_signal.interrupt();
            }
        }
        std::future::pending::<()>().await;
    };
    tokio::pin!(signal_listener);
    tokio::pin!(fut);

    let timeout_sleep = async {
        if let Some(dur) = timeout {
            let sleep_res =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tokio::time::sleep(dur)));
            if let Ok(fut) = sleep_res {
                SafeFuture::new(fut).await;
                true
            } else {
                std::future::pending::<()>().await;
                false
            }
        } else {
            std::future::pending::<()>().await;
            false
        }
    };
    tokio::pin!(timeout_sleep);

    let cancelled_fut = cancel.cancelled();
    tokio::pin!(cancelled_fut);

    let cancelled = if cancel.is_cancelled() {
        true
    } else {
        tokio::select! {
            res = &mut fut => {
                return res;
            }
            () = &mut signal_listener => {
                unreachable!("signal_listener never returns Ok(())");
            }
            timed_out = &mut timeout_sleep => {
                if timed_out {
                    let formatted = crate::app::format_duration(timeout.unwrap_or(std::time::Duration::ZERO));
                    return Err(Failure::execution(format!("timed out after {formatted}")));
                }
                false
            }
            () = &mut cancelled_fut => {
                true
            }
        }
    };

    if cancelled {
        let grace_sleep = async {
            let sleep_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::time::sleep(grace_period)
            }));
            if let Ok(fut) = sleep_res {
                SafeFuture::new(fut).await;
            } else {
                std::future::pending::<()>().await;
            }
        };
        tokio::pin!(grace_sleep);
        let aborted_fut = cancel.aborted();
        tokio::pin!(aborted_fut);

        tokio::select! {
            res = &mut fut => {
                res
            }
            () = &mut signal_listener => {
                unreachable!("signal_listener never returns Ok(())");
            }
            timed_out = &mut timeout_sleep => {
                if timed_out {
                    let formatted = crate::app::format_duration(timeout.unwrap_or(std::time::Duration::ZERO));
                    return Err(Failure::execution(format!("timed out after {formatted}")));
                }
                Err(Failure::cancelled("operation aborted by interrupt"))
            }
            () = &mut aborted_fut => {
                Err(Failure::cancelled("operation aborted by interrupt"))
            }
            () = &mut grace_sleep => {
                Err(Failure::cancelled("operation aborted by interrupt"))
            }
        }
    } else {
        unreachable!("phase 1 handles non-cancelled completion")
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
        let handle_exec = TokioExecutor::from_handle(rt.handle().clone())
            .grace_period(std::time::Duration::from_millis(50));
        let res_handle = handle_exec.run(Box::pin(async { Ok::<_, Failure>(3) }), Cancel::new());
        assert_eq!(res_handle.unwrap(), 3);
    }

    #[test]
    fn cancel_interrupt_and_abort_state_machine() {
        let cancel = Cancel::new();
        assert!(!cancel.is_cancelled());
        assert!(!cancel.is_aborted());
        assert_eq!(cancel.interrupt_count(), 0);
        assert_eq!(cancel.to_string(), "active");

        // First interrupt trips cancel (cooperative cancellation)
        cancel.interrupt();
        assert!(cancel.is_cancelled());
        assert!(!cancel.is_aborted());
        assert_eq!(cancel.interrupt_count(), 1);
        assert_eq!(cancel.to_string(), "cancelled");

        // Second interrupt trips abort (hard abort)
        cancel.interrupt();
        assert!(cancel.is_cancelled());
        assert!(cancel.is_aborted());
        assert_eq!(cancel.interrupt_count(), 2);
        assert_eq!(cancel.to_string(), "aborted");

        // Third interrupt increments count
        cancel.interrupt();
        assert_eq!(cancel.interrupt_count(), 3);

        // Abort method sets aborted directly
        let cancel2 = Cancel::new();
        cancel2.abort();
        assert!(cancel2.is_cancelled());
        assert!(cancel2.is_aborted());
        assert_eq!(cancel2.to_string(), "aborted");

        let debug_str = format!("{cancel2:?}");
        assert!(debug_str.contains("is_cancelled: true"));
        assert!(debug_str.contains("is_aborted: true"));

        let (cx, _) = RunCx::for_test();
        assert!(!cx.is_aborted());
        cx.request_interrupt();
        assert!(cx.is_cancelled());
        assert!(!cx.is_aborted());
        cx.request_interrupt();
        assert!(cx.is_aborted());

        let (cx2, _) = RunCx::for_test();
        cx2.request_abort();
        assert!(cx2.is_aborted());
    }

    #[test]
    fn aborted_future_polling_behavior() {
        let cancel = Cancel::new();
        let mut fut = cancel.aborted();

        let pending = std::future::Future::poll(
            std::pin::pin!(&mut fut),
            &mut Context::from_waker(Waker::noop()),
        );
        assert!(pending.is_pending());

        // First interrupt only trips cancel, not abort
        cancel.interrupt();
        let still_pending = std::future::Future::poll(
            std::pin::pin!(&mut fut),
            &mut Context::from_waker(Waker::noop()),
        );
        assert!(still_pending.is_pending());

        // Second interrupt trips abort
        cancel.interrupt();
        let ready = std::future::Future::poll(
            std::pin::pin!(&mut fut),
            &mut Context::from_waker(Waker::noop()),
        );
        assert!(ready.is_ready());

        let mut immediate = cancel.aborted();
        let ready_imm = std::future::Future::poll(
            std::pin::pin!(&mut immediate),
            &mut Context::from_waker(Waker::noop()),
        );
        assert!(ready_imm.is_ready());
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn run_with_cancellation_normal_success_and_failure() {
        let cancel = Cancel::new();
        let ok_res = run_with_cancellation(
            async { Ok::<_, Failure>("success") },
            cancel.clone(),
            std::time::Duration::from_millis(50),
            None,
        )
        .await;
        assert_eq!(ok_res.unwrap(), "success");

        let err_res = run_with_cancellation(
            async { Err::<String, _>(Failure::validation("invalid input")) },
            cancel,
            std::time::Duration::from_millis(50),
            None,
        )
        .await;
        assert!(err_res.is_err());
        assert_eq!(err_res.unwrap_err().message(), "invalid input");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn run_with_cancellation_cooperative_cancel_preserves_message() {
        let cancel = Cancel::new();
        let cancel_clone = cancel.clone();

        let res = run_with_cancellation(
            async move {
                cancel_clone.interrupt(); // First Ctrl-C
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                Err::<String, _>(Failure::cancelled("custom clean exit message"))
            },
            cancel,
            std::time::Duration::from_millis(100),
            None,
        )
        .await;

        assert!(res.is_err());
        let failure = res.unwrap_err();
        assert_eq!(failure.kind(), crate::failure::FailureKind::Cancelled);
        assert_eq!(failure.message(), "custom clean exit message");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn run_with_cancellation_second_interrupt_hard_aborts() {
        let cancel = Cancel::new();
        let cancel_clone = cancel.clone();

        let res = run_with_cancellation(
            async move {
                cancel_clone.interrupt(); // 1st Ctrl-C
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                cancel_clone.interrupt(); // 2nd Ctrl-C -> immediate hard abort
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                Ok::<_, Failure>("unreachable")
            },
            cancel,
            std::time::Duration::from_millis(1000),
            None,
        )
        .await;

        assert!(res.is_err());
        let failure = res.unwrap_err();
        assert_eq!(failure.kind(), crate::failure::FailureKind::Cancelled);
        assert_eq!(failure.message(), "operation aborted by interrupt");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn run_with_cancellation_grace_period_expiration_hard_aborts() {
        let cancel = Cancel::new();
        let cancel_clone = cancel.clone();

        let res = run_with_cancellation(
            async move {
                cancel_clone.interrupt(); // 1st Ctrl-C
                // Task hangs longer than the 50ms grace period without second interrupt
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                Ok::<_, Failure>("unreachable")
            },
            cancel,
            std::time::Duration::from_millis(50),
            None,
        )
        .await;

        assert!(res.is_err());
        let failure = res.unwrap_err();
        assert_eq!(failure.kind(), crate::failure::FailureKind::Cancelled);
        assert_eq!(failure.message(), "operation aborted by interrupt");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn run_with_cancellation_timeout_expiration_aborts_with_execution_failure() {
        let cancel = Cancel::new();

        let res = run_with_cancellation(
            async {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                Ok::<_, Failure>("finished")
            },
            cancel,
            std::time::Duration::from_millis(50),
            Some(std::time::Duration::from_millis(20)),
        )
        .await;

        assert!(res.is_err());
        let failure = res.unwrap_err();
        assert_eq!(failure.kind(), crate::failure::FailureKind::Execution);
        assert_eq!(failure.message(), "timed out after 20ms");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn run_with_cancellation_completes_before_timeout() {
        let cancel = Cancel::new();

        let res = run_with_cancellation(
            async {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                Ok::<_, Failure>("in time")
            },
            cancel,
            std::time::Duration::from_millis(50),
            Some(std::time::Duration::from_millis(100)),
        )
        .await;

        assert_eq!(res.unwrap(), "in time");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(start_paused = true)]
    async fn run_with_cancellation_pre_cancelled_and_pre_aborted() {
        let pre_aborted = Cancel::new();
        pre_aborted.abort();
        let res_aborted = run_with_cancellation(
            async { Ok::<_, Failure>("never") },
            pre_aborted,
            std::time::Duration::from_millis(50),
            None,
        )
        .await;
        assert_eq!(
            res_aborted.unwrap_err().message(),
            "operation aborted by interrupt"
        );

        let pre_cancelled = Cancel::new();
        pre_cancelled.cancel();
        let res_cancelled = run_with_cancellation(
            async {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                Err::<String, _>(Failure::cancelled("saved"))
            },
            pre_cancelled,
            std::time::Duration::from_millis(50),
            None,
        )
        .await;
        assert_eq!(res_cancelled.unwrap_err().message(), "saved");
    }
}
