//! Writing what went wrong, and answering with what it costs.
//!
//! Every application ends the same way: a [`Failure`] is written on the diagnostic
//! stream and costs what its kind costs, and `clap`'s own verdict on the invocation is
//! written where `clap` says — help and the version on the data stream as a success, a
//! usage error with its usage on the diagnostic stream as a usage failure. [`render`]
//! is that mapping, once, against the application's [`ExitPolicy`], so the numbers stay
//! the application's (decision-3) while the code that applies them stops being
//! rewritten in every `main`.
//!
//! ```
//! use clifacet::Failure;
//! use clifacet::contract::ExitPolicy;
//! use clifacet::report;
//!
//! let policy = ExitPolicy::standard();
//! let (mut out, mut err) = (Vec::new(), Vec::new());
//!
//! let code = report::render(
//!     "greet",
//!     &Failure::validation("--name must not be blank"),
//!     &policy,
//!     &mut out,
//!     &mut err,
//! );
//!
//! assert_eq!(code, policy.validation());
//! assert_eq!(String::from_utf8(err).unwrap(), "greet: --name must not be blank\n");
//! assert!(out.is_empty());
//! ```
//!
//! A failure is written as `program: message`. Its kind is not repeated in the text:
//! the exit code carries it, and the message is already written for a person.

use std::io::Write;

use crate::contract::ExitPolicy;
use crate::failure::Failure;

/// Something [`render`] can write: a failed run, or `clap`'s answer to the invocation.
///
/// Built from either with `From`, so a call site passes `&failure` or `&error` as it
/// has it.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Report<'a> {
    /// A run that failed validation or execution, or that the user cancelled.
    Failure(&'a Failure),
    /// `clap`'s verdict instead of a parse: a usage error, or the help or version
    /// that was asked for.
    Clap(&'a clap::Error),
}

impl<'a> From<&'a Failure> for Report<'a> {
    fn from(failure: &'a Failure) -> Self {
        Self::Failure(failure)
    }
}

impl<'a> From<&'a clap::Error> for Report<'a> {
    fn from(error: &'a clap::Error) -> Self {
        Self::Clap(error)
    }
}

/// Writes `report` on the stream it belongs on and answers with the code it costs.
///
/// - A [`Failure`] is written as `program: message` on `err`, and costs
///   [`ExitPolicy::for_failure`].
/// - A `clap` usage error is written as `clap` renders it — its message, the usage
///   line and the hint to try `--help` — on `err`, and costs [`ExitPolicy::usage`].
/// - Help and the version are the output that was asked for: written on `out`, and
///   costing [`ExitPolicy::success`].
///
/// A stream that cannot be written — a closed pipe, usually — is ignored: the code is
/// the answer either way, and failing to report a failure has nowhere left to go.
pub fn render<'a>(
    program: &str,
    report: impl Into<Report<'a>>,
    policy: &ExitPolicy,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match report.into() {
        Report::Failure(failure) => {
            let _ = writeln!(err, "{program}: {}", failure.message());
            policy.for_failure(failure)
        }
        // `clap` decides the stream as well as the outcome: help is the result the
        // caller asked for, while a usage error is a diagnostic.
        Report::Clap(error) if error.use_stderr() => {
            let _ = write!(err, "{error}");
            policy.usage()
        }
        Report::Clap(error) => {
            let _ = write!(out, "{error}");
            policy.success()
        }
    }
}

/// Standardized JSON error payload emitted on `stderr` when `--json` is active (decision-25).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErrorPayload {
    /// Status indicator, always `"error"`.
    pub status: String,
    /// Detailed failure metadata.
    pub error: ErrorDetail,
}

/// Details of the error in an [`ErrorPayload`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErrorDetail {
    /// Failure category (e.g. `"validation"`, `"execution"`, `"cancelled"`, `"usage"`).
    pub kind: String,
    /// Exit code associated with this failure.
    pub code: i32,
    /// Human-readable message describing the failure.
    pub message: String,
}

/// Writes `report` as a standardized JSON error payload on `err` (or data on `out` for help/version)
/// and answers with the code it costs.
///
/// When the failure or usage error is written to `err`, it conforms to the decision-25 schema:
/// ```json
/// {
///   "status": "error",
///   "error": {
///     "kind": "validation",
///     "code": 2,
///     "message": "..."
///   }
/// }
/// ```
pub fn render_json<'a>(
    _program: &str,
    report: impl Into<Report<'a>>,
    policy: &ExitPolicy,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match report.into() {
        Report::Failure(failure) => {
            let code = policy.for_failure(failure);
            let kind = match failure.kind() {
                crate::failure::FailureKind::Validation => "validation",
                crate::failure::FailureKind::Execution => "execution",
                crate::failure::FailureKind::Cancelled => "cancelled",
            };
            write_json_error(err, kind, code, failure.message());
            code
        }
        Report::Clap(error) if error.use_stderr() => {
            let code = policy.usage();
            let kind = match error.kind() {
                clap::error::ErrorKind::ValueValidation
                | clap::error::ErrorKind::MissingRequiredArgument
                | clap::error::ErrorKind::MissingSubcommand
                | clap::error::ErrorKind::WrongNumberOfValues
                | clap::error::ErrorKind::InvalidValue => "validation",
                _ => "usage",
            };
            let raw = error.to_string();
            let mut message = raw.trim();
            if let Some(idx) = message.find("\nUsage:") {
                message = &message[..idx];
            } else if let Some(idx) = message.find("\n\nUsage:") {
                message = &message[..idx];
            } else if let Some(idx) = message.find("\n\nFor more information") {
                message = &message[..idx];
            }
            let message = message.strip_prefix("error: ").unwrap_or(message).trim();
            write_json_error(err, kind, code, message);
            code
        }
        Report::Clap(error) => {
            let _ = write!(out, "{error}");
            policy.success()
        }
    }
}

fn write_json_error(err: &mut dyn Write, kind: &str, code: i32, message: &str) {
    #[cfg(feature = "serde")]
    {
        let payload = ErrorPayload {
            status: "error".to_owned(),
            error: ErrorDetail {
                kind: kind.to_owned(),
                code,
                message: message.to_owned(),
            },
        };
        if let Ok(json) = serde_json::to_string_pretty(&payload) {
            let _ = writeln!(err, "{json}");
            return;
        }
    }
    let _ = writeln!(
        err,
        "{{\n  \"status\": \"error\",\n  \"error\": {{\n    \"kind\": \"{}\",\n    \"code\": {},\n    \"message\": \"{}\"\n  }}\n}}",
        escape_str(kind),
        code,
        escape_str(message)
    );
}

fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}
