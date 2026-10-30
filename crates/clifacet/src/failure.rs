//! Library-owned failure categories for the UI-free command core.
//!
//! A command can fail in two places, and the difference matters to a caller: the
//! input can be rejected before the command ever runs ([`Failure::validation`]),
//! or the command can run and fail ([`Failure::execution`]). Both travel as one
//! type so every presentation layer — flags today, prompts and a TUI later —
//! reports them the same way.
//!
//! Exit codes are deliberately *not* here. decision-3 keeps output and exit
//! mapping application-owned, so this type answers "what kind of failure is
//! this?" and the application decides what that costs.

use std::error::Error;
use std::fmt;

/// What kind of failure a [`Failure`] records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FailureKind {
    /// The input failed validation, so the command did not run.
    Validation,
    /// The command ran and could not complete.
    Execution,
}

/// A command failure, with the kind and a caller-facing message.
///
/// The message is written for a person reading a diagnostic, not for parsing.
/// Machine-readable output belongs on the application's data stream, which a
/// failure never writes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    kind: FailureKind,
    message: String,
}

impl Failure {
    /// Records that the input failed validation.
    ///
    /// The command was never invoked, so nothing it owns has changed.
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Validation,
            message: message.into(),
        }
    }

    /// Records that the command ran and could not complete.
    #[must_use]
    pub fn execution(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Execution,
            message: message.into(),
        }
    }

    /// The category of this failure.
    ///
    /// Applications map this to their own exit codes and messages; see
    /// [`crate::example`] for a worked mapping.
    #[must_use]
    pub const fn kind(&self) -> FailureKind {
        self.kind
    }

    /// The caller-facing message, without the diagnostic prefix an application adds.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for Failure {}
