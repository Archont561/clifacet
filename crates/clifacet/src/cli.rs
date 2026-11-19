//! The [`Cli`] and [`FromCli`] traits defining the application lifecycle contract (decision-24).

use std::ops::Deref;
use std::sync::{Arc, OnceLock};

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

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
}
