//! The [`Cli`] and [`FromCli`] traits defining the application lifecycle contract (decision-24).

use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use crate::command::Plan;
use crate::failure::Failure;

/// The application lifecycle contract: context initialization and execution hooks.
pub trait Cli: Sized {
    /// The application context (services, connection pools, configuration).
    type Context;

    /// Lazy context factory.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if the application context cannot be constructed.
    ///
    /// # Guarantee
    ///
    /// This method is never invoked on `--help`, `--version`, or argument parsing errors.
    fn init_context(&self) -> Result<Self::Context, Failure>;

    /// Hook: Runs immediately before command execution starts.
    ///
    /// # Errors
    ///
    /// If an error is returned, command execution halts and [`after_failure`](Self::after_failure) is invoked.
    fn before_run(&self, _cmd: &str, _cx: &Self::Context) -> Result<(), Failure> {
        Ok(())
    }

    /// Hook: Runs after a command successfully finishes.
    ///
    /// # Errors
    ///
    /// If an error is returned, the run fails and [`after_failure`](Self::after_failure) is invoked.
    fn after_success(&self, _cmd: &str, _cx: &Self::Context) -> Result<(), Failure> {
        Ok(())
    }

    /// Hook: Runs if a command returns an error, `before_run` fails, or context creation fails.
    ///
    /// `cx` is `None` if context creation itself failed.
    fn after_failure(&self, _cmd: &str, _err: &Failure, _cx: Option<&Self::Context>) {
        // Default: no-op
    }
}

/// Constructs an application context or component from parsed CLI root options.
pub trait FromCli<C: Cli>: Sized {
    /// Constructs the context from `cli`.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if the context cannot be constructed.
    fn from_cli(cli: &C) -> Result<Self, Failure>;
}

/// Thread-safe cell for lazily initialized application context.
#[derive(Clone)]
pub struct ContextCell<T>(Arc<OnceLock<T>>);

impl<T> Default for ContextCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ContextCell<T> {
    /// Creates an empty context cell.
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(OnceLock::new()))
    }

    /// Stores the initialized context into this cell.
    ///
    /// # Errors
    ///
    /// Returns `Err(value)` if the cell was already initialized.
    pub fn set(&self, value: T) -> Result<(), T> {
        self.0.set(value)
    }

    /// Returns a reference to the initialized context, or `None` if uninitialized.
    #[must_use]
    pub fn get(&self) -> Option<&T> {
        self.0.get()
    }

    /// Returns `true` if the context has been initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.0.get().is_some()
    }
}

impl<T> From<T> for ContextCell<T> {
    fn from(value: T) -> Self {
        let cell = Self::new();
        let _ = cell.set(value);
        cell
    }
}

impl<T> Deref for ContextCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
            .get()
            .expect("clifacet: context accessed before lazy initialization")
    }
}

impl<T> AsRef<T> for ContextCell<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for ContextCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.get() {
            Some(val) => f.debug_tuple("ContextCell").field(val).finish(),
            None => f
                .debug_tuple("ContextCell")
                .field(&"<uninitialized>")
                .finish(),
        }
    }
}

#[doc(hidden)]
pub mod auto_wire {
    use super::{Cli, FromCli};
    use crate::failure::Failure;
    use std::marker::PhantomData;

    pub struct AutoWire<T>(pub PhantomData<T>);

    pub trait AutoWireDefault<T: Default> {
        fn init_ctx<C>(&self, _cli: &C) -> Result<T, Failure> {
            Ok(T::default())
        }
    }

    impl<T: Default> AutoWireDefault<T> for AutoWire<T> {}

    pub trait AutoWireFromCli<T: FromCli<C>, C: Cli> {
        fn init_ctx(&self, cli: &C) -> Result<T, Failure> {
            <T as FromCli<C>>::from_cli(cli)
        }
    }

    impl<T: FromCli<C>, C: Cli> AutoWireFromCli<T, C> for &AutoWire<T> {}
}

#[doc(hidden)]
pub mod check_dry_run {
    pub trait FallbackIsDryRun {
        fn __clifacet_is_dry_run(&self) -> bool {
            false
        }
    }

    impl<T: ?Sized> FallbackIsDryRun for T {}
}

#[doc(hidden)]
pub mod check_json {
    pub trait FallbackIsJson {
        fn __clifacet_is_json(&self) -> bool {
            false
        }
    }

    impl<T: ?Sized> FallbackIsJson for T {}
}

#[doc(hidden)]
pub mod auto_wire_json {
    use crate::failure::Failure;

    pub trait FallbackFormatOutcome {
        fn format_outcome(self, _is_dry_run: bool, _is_json: bool) -> Result<String, Failure>;
    }

    impl<T: std::fmt::Display> FallbackFormatOutcome for T {
        fn format_outcome(self, _is_dry_run: bool, _is_json: bool) -> Result<String, Failure> {
            Ok(self.to_string())
        }
    }
}

#[doc(hidden)]
pub mod extract {
    use std::marker::PhantomData;

    pub struct ExtractWrap<T>(pub PhantomData<T>);

    pub trait ExtractAsRef<T: Clone, P: AsRef<T>> {
        fn extract(&self, parent: &P) -> T {
            parent.as_ref().clone()
        }
    }
    impl<T: Clone, P: AsRef<T>> ExtractAsRef<T, P> for ExtractWrap<T> {}

    pub trait ExtractFrom<T, P> {
        fn extract(&self, parent: &P) -> T;
    }
    impl<T, P> ExtractFrom<T, P> for &ExtractWrap<T>
    where
        T: for<'a> From<&'a P>,
    {
        fn extract(&self, parent: &P) -> T {
            T::from(parent)
        }
    }
}

/// Trait for converting error types into [`Failure`].
pub trait IntoFailure {
    /// Converts into [`Failure`].
    fn into_failure(self) -> Failure;
}

impl IntoFailure for Failure {
    fn into_failure(self) -> Failure {
        self
    }
}

impl IntoFailure for String {
    fn into_failure(self) -> Failure {
        Failure::execution(self)
    }
}

impl IntoFailure for &str {
    fn into_failure(self) -> Failure {
        Failure::execution(self)
    }
}

impl IntoFailure for std::io::Error {
    fn into_failure(self) -> Failure {
        Failure::execution(self.to_string())
    }
}

impl IntoFailure for Box<dyn std::error::Error + Send + Sync> {
    fn into_failure(self) -> Failure {
        Failure::execution(self.to_string())
    }
}

impl IntoFailure for Box<dyn std::error::Error> {
    fn into_failure(self) -> Failure {
        Failure::execution(self.to_string())
    }
}

/// Types that can be the successful payload of an [`IntoOutcome`].
pub trait OutcomePayload: Sized {
    /// The formatted representation of the payload.
    type Displayed: std::fmt::Display;

    /// Formats or converts the payload into its displayed form.
    fn into_displayed(self) -> Self::Displayed;
}

impl OutcomePayload for () {
    type Displayed = String;
    fn into_displayed(self) -> String {
        String::new()
    }
}

impl OutcomePayload for String {
    type Displayed = String;
    fn into_displayed(self) -> String {
        self
    }
}

impl OutcomePayload for &'static str {
    type Displayed = &'static str;
    fn into_displayed(self) -> &'static str {
        self
    }
}

macro_rules! impl_outcome_payload {
    ($($t:ty),*) => {
        $(
            impl OutcomePayload for $t {
                type Displayed = $t;
                fn into_displayed(self) -> $t {
                    self
                }
            }
        )*
    };
}
impl_outcome_payload!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, char
);

/// Converts command method return types into the runner outcome.
pub trait IntoOutcome {
    /// The formatted output type.
    type Output: std::fmt::Display;

    /// Converts into `Result<Self::Output, Failure>`.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if the command failed.
    fn into_outcome(self) -> Result<Self::Output, Failure>;
}

impl IntoOutcome for () {
    type Output = String;
    fn into_outcome(self) -> Result<Self::Output, Failure> {
        Ok(String::new())
    }
}

impl IntoOutcome for String {
    type Output = String;
    fn into_outcome(self) -> Result<Self::Output, Failure> {
        Ok(self)
    }
}

impl IntoOutcome for &'static str {
    type Output = &'static str;
    fn into_outcome(self) -> Result<Self::Output, Failure> {
        Ok(self)
    }
}

impl<T: OutcomePayload, E: IntoFailure> IntoOutcome for Result<T, E> {
    type Output = T::Displayed;
    fn into_outcome(self) -> Result<Self::Output, Failure> {
        self.map(OutcomePayload::into_displayed)
            .map_err(IntoFailure::into_failure)
    }
}

/// An outcome payload that serializes as structured JSON, or a renderer when `T = ()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Json<T = ()>(pub T);

/// A renderer that writes output directly as structured JSON.
pub type JsonRenderer = Json<()>;

impl<T> Json<T> {
    /// Wraps `value` to be formatted as JSON.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Unwraps the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Formats the outcome as a JSON string.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if serialization fails.
    #[cfg(feature = "serde")]
    pub fn format_outcome(self, _is_dry_run: bool, _is_json: bool) -> Result<String, Failure>
    where
        T: serde::Serialize,
    {
        serde_json::to_string_pretty(&self.0)
            .map_err(|e| Failure::execution(format!("cannot serialize output: {e}")))
    }
}

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> std::fmt::Display for Json<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(&self.0) {
            Ok(json) => f.write_str(&json),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> OutcomePayload for Json<T> {
    type Displayed = String;
    fn into_displayed(self) -> String {
        serde_json::to_string_pretty(&self.0).unwrap_or_default()
    }
}

#[cfg(feature = "serde")]
impl OutcomePayload for serde_json::Value {
    type Displayed = String;
    fn into_displayed(self) -> String {
        serde_json::to_string_pretty(&self).unwrap_or_default()
    }
}

/// A command outcome holding structured data for `--json` and human-readable text for plain rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structured<T> {
    /// The structured data payload.
    pub data: T,
    /// The human-readable display string.
    pub display: String,
}

impl<T> Structured<T> {
    /// Creates a new outcome with its structured data and human-readable text.
    pub fn new(data: T, display: impl Into<String>) -> Self {
        Self {
            data,
            display: display.into(),
        }
    }

    /// Formats the outcome depending on whether JSON mode is requested.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if serialization fails.
    #[cfg(feature = "serde")]
    pub fn format_outcome(self, _is_dry_run: bool, is_json: bool) -> Result<String, Failure>
    where
        T: serde::Serialize,
    {
        if is_json {
            serde_json::to_string_pretty(&self.data)
                .map_err(|e| Failure::execution(format!("cannot serialize output: {e}")))
        } else {
            Ok(self.display)
        }
    }

    /// Formats the outcome when serde is not enabled.
    ///
    /// # Errors
    ///
    /// Always succeeds.
    #[cfg(not(feature = "serde"))]
    pub fn format_outcome(self, _is_dry_run: bool, _is_json: bool) -> Result<String, Failure> {
        Ok(self.display)
    }
}

impl<T> std::fmt::Display for Structured<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for Structured<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> OutcomePayload for Structured<T> {
    type Displayed = Self;
    fn into_displayed(self) -> Self {
        self
    }
}

/// A planned outcome for commands returning [`Plan`].
pub struct PlannedOutcome<'a, T> {
    plan: Plan<'a, T>,
}

impl<'a, T> PlannedOutcome<'a, T> {
    /// Wraps a [`Plan`] into a [`PlannedOutcome`].
    #[must_use]
    pub fn new(plan: Plan<'a, T>) -> Self {
        Self { plan }
    }

    /// Formats the planned outcome depending on whether dry-run and/or JSON formatting
    /// are requested.
    ///
    /// In dry-run mode, the plan steps are rendered without executing the underlying work closure.
    /// In normal mode, the underlying work closure is executed and the result is rendered.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if plan execution or serialization fails.
    pub fn format_outcome(self, is_dry_run: bool, is_json: bool) -> Result<String, Failure>
    where
        T: OutcomePayload,
    {
        use auto_wire_json::FallbackFormatOutcome as _;

        if is_dry_run {
            let steps = self.plan.describe();
            #[cfg(feature = "serde")]
            if is_json {
                return serde_json::to_string_pretty(&serde_json::json!({
                    "dry_run": true,
                    "steps": self.plan.steps(),
                }))
                .map_err(|e| Failure::execution(format!("cannot format dry-run JSON: {e}")));
            }
            #[cfg(not(feature = "serde"))]
            let _ = is_json;
            Ok(steps)
        } else {
            let res = self.plan.execute()?;
            res.into_displayed().format_outcome(is_dry_run, is_json)
        }
    }
}

impl<T> std::fmt::Display for PlannedOutcome<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.plan.describe())
    }
}

impl<'a, T: OutcomePayload, E: IntoFailure> IntoOutcome for Result<Plan<'a, T>, E> {
    type Output = PlannedOutcome<'a, T>;

    fn into_outcome(self) -> Result<Self::Output, Failure> {
        self.map(PlannedOutcome::new)
            .map_err(IntoFailure::into_failure)
    }
}

impl<'a, T: OutcomePayload> IntoOutcome for Plan<'a, T> {
    type Output = PlannedOutcome<'a, T>;

    fn into_outcome(self) -> Result<Self::Output, Failure> {
        Ok(PlannedOutcome::new(self))
    }
}

/// Context helper for logging and dry-run action dispatching to `stderr`.
///
/// Can be included directly as a context field in `#[clifacet::app]` controllers:
///
/// ```rust,ignore
/// use clifacet::{app, commands, ActionLog, Failure, Globals};
///
/// #[app(name = "demo", commands)]
/// struct DemoApp {
///     #[command(flatten)]
///     #[clifacet(skip)]
///     globals: Globals,
///
///     #[clifacet(context)]
///     ctx: ActionLog,
/// }
///
/// #[commands]
/// impl DemoApp {
///     pub fn deploy(&self, target: String) -> Result<String, Failure> {
///         self.ctx.log(format!("deploying to {target}"));
///         self.ctx.action(format!("upload artifacts to {target}"), || {
///             // real side-effect
///             Ok(())
///         })?;
///         Ok(format!("deployed to {target}"))
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActionLog {
    dry_run: bool,
}

impl ActionLog {
    /// Creates an `ActionLog` with explicit dry-run state.
    #[must_use]
    pub const fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Returns `true` if dry-run mode is active.
    #[must_use]
    pub const fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Writes a log message to `stderr`, prefixed with `[DRY-RUN]` when dry-run is active.
    pub fn log(&self, message: impl std::fmt::Display) {
        if self.dry_run {
            eprintln!("[DRY-RUN] {message}");
        } else {
            eprintln!("{message}");
        }
    }

    /// Logs `description` to `stderr`, and executes `action` only when dry-run is not active.
    ///
    /// Returns `Ok(Some(T))` on real execution, or `Ok(None)` when dry-run is active.
    ///
    /// # Errors
    ///
    /// Propagates any [`Failure`] returned by `action`.
    pub fn action<T>(
        &self,
        description: impl std::fmt::Display,
        action: impl FnOnce() -> Result<T, Failure>,
    ) -> Result<Option<T>, Failure> {
        self.log(description);
        if self.dry_run {
            Ok(None)
        } else {
            action().map(Some)
        }
    }
}

impl<C: Cli> FromCli<C> for ActionLog {
    fn from_cli(cli: &C) -> Result<Self, Failure> {
        use crate::cli::check_dry_run::FallbackIsDryRun as _;
        Ok(ActionLog::new(cli.__clifacet_is_dry_run()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn context_cell_operations() {
        let empty: ContextCell<i32> = ContextCell::new();
        assert!(!empty.is_initialized());
        assert_eq!(empty.get(), None);
        assert_eq!(format!("{empty:?}"), "ContextCell(\"<uninitialized>\")");

        let cell = ContextCell::from(42);
        assert!(cell.is_initialized());
        assert_eq!(cell.get(), Some(&42));
        assert_eq!(*cell, 42);
        assert_eq!(cell.as_ref(), &42);
        assert_eq!(format!("{cell:?}"), "ContextCell(42)");
    }

    #[test]
    fn into_failure_conversions() {
        let f = Failure::execution("err");
        let converted = f.clone().into_failure();
        assert_eq!(converted.to_string(), "err");

        let s = "string error".to_string();
        assert_eq!(s.into_failure().to_string(), "string error");

        let s_ref = "str error";
        assert_eq!(s_ref.into_failure().to_string(), "str error");

        let io_err = io::Error::other("io error");
        assert_eq!(io_err.into_failure().to_string(), "io error");

        let boxed_sync: Box<dyn std::error::Error + Send + Sync> = "box send sync".into();
        assert_eq!(boxed_sync.into_failure().to_string(), "box send sync");

        let boxed: Box<dyn std::error::Error> = "box err".into();
        assert_eq!(boxed.into_failure().to_string(), "box err");
    }

    #[test]
    fn outcome_payload_and_conversions() {
        let static_str = "hello";
        assert_eq!(OutcomePayload::into_displayed(static_str), "hello");
        assert_eq!(IntoOutcome::into_outcome(static_str).unwrap(), "hello");

        let res_ok: Result<&'static str, &'static str> = Ok("ok");
        assert_eq!(res_ok.into_outcome().unwrap(), "ok");

        let res_err: Result<&'static str, &'static str> = Err("err");
        assert!(res_err.into_outcome().is_err());

        let unit_outcome: Result<String, Failure> = ().into_outcome();
        assert_eq!(unit_outcome.unwrap(), "");

        let string_outcome: Result<String, Failure> = "str".to_string().into_outcome();
        assert_eq!(string_outcome.unwrap(), "str");
    }

    #[test]
    fn extract_helpers() {
        use crate::cli::extract::{ExtractAsRef, ExtractFrom, ExtractWrap};

        struct Parent(String);
        impl AsRef<String> for Parent {
            fn as_ref(&self) -> &String {
                &self.0
            }
        }
        impl<'a> From<&'a Parent> for usize {
            fn from(p: &'a Parent) -> Self {
                p.0.len()
            }
        }

        let wrap: ExtractWrap<String> = ExtractWrap(std::marker::PhantomData);
        let p = Parent("hello".into());
        assert_eq!(ExtractAsRef::extract(&wrap, &p), "hello".to_string());

        let wrap_ref = ExtractWrap::<usize>(std::marker::PhantomData);
        assert_eq!(ExtractFrom::extract(&&wrap_ref, &p), 5);
    }

    #[test]
    fn planned_outcome_dry_run_and_execution() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = Arc::clone(&executed);

        let plan = Plan::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
            Ok("task completed".to_string())
        })
        .step("step 1: validate")
        .step("step 2: mutate");

        let outcome = PlannedOutcome::new(plan);
        // Dry-run mode: does not execute closure
        let dry_run_text = outcome.format_outcome(true, false).unwrap();
        assert!(!executed.load(Ordering::SeqCst));
        assert!(dry_run_text.contains("step 1: validate"));
        assert!(dry_run_text.contains("step 2: mutate"));

        // Real mode: executes closure
        let executed_clone2 = Arc::clone(&executed);
        let real_plan = Plan::new(move || {
            executed_clone2.store(true, Ordering::SeqCst);
            Ok("task completed".to_string())
        })
        .step("step 1: validate");

        let real_outcome = PlannedOutcome::new(real_plan);
        let real_text = real_outcome.format_outcome(false, false).unwrap();
        assert!(executed.load(Ordering::SeqCst));
        assert_eq!(real_text, "task completed");
    }

    #[test]
    fn action_log_operations() {
        let dry_log = ActionLog::new(true);
        assert!(dry_log.is_dry_run());

        let executed = Arc::new(AtomicBool::new(false));
        let exec_clone = Arc::clone(&executed);

        let res = dry_log.action("dry action", || {
            exec_clone.store(true, Ordering::SeqCst);
            Ok("done")
        });
        assert_eq!(res.unwrap(), None);
        assert!(!executed.load(Ordering::SeqCst));

        let real_log = ActionLog::new(false);
        assert!(!real_log.is_dry_run());

        let res_real = real_log.action("real action", || {
            executed.store(true, Ordering::SeqCst);
            Ok("done")
        });
        assert_eq!(res_real.unwrap(), Some("done"));
        assert!(executed.load(Ordering::SeqCst));
    }
}
