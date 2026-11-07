//! What each outcome costs, and the one place an outcome is written.

use std::io::Write;

use clifacet::Failure;
use clifacet::contract::ExitPolicy;

/// The exit codes this application gives its outcomes.
///
/// The numbers are the application's (decision-3); the categories are the library's.
/// [`ExitPolicy::standard`] is that mapping in one readable place, and the constants
/// below are names for it so a test can point at the outcome it means.
pub const EXIT: ExitPolicy = ExitPolicy::standard();

/// The process exited after doing what was asked.
pub const EXIT_SUCCESS: i32 = EXIT.success();

/// The command ran and could not complete.
pub const EXIT_EXECUTION_FAILED: i32 = EXIT.execution();

/// The input failed validation before the command ran.
pub const EXIT_VALIDATION_FAILED: i32 = EXIT.validation();

/// `clap` rejected the arguments, or the user asked for help or the version.
///
/// The same value as validation on purpose: both mean "nothing ran, fix the
/// invocation", and `clap` itself documents 2 for usage errors.
pub const EXIT_USAGE: i32 = EXIT.usage();

/// The user cancelled an interactive flow, so nothing ran.
///
/// `130` is the conventional shell code for interruption, and it is deliberately
/// not `2`: a cancellation is a decision, not a mistake in the invocation.
pub const EXIT_CANCELLED: i32 = EXIT.cancelled();

/// The exit code a failure costs this application.
///
/// The numbers are the application's, not the library's (decision-3), and each
/// failure category gets its own: cancelled, execution, and anything else — mode
/// refusals and validation — meaning "nothing ran, fix the invocation". The mapping
/// itself lives in [`EXIT`], so it can be read in one place and asserted as a policy.
#[must_use]
pub fn exit_code(failure: &Failure) -> i32 {
    EXIT.for_failure(failure)
}

/// Writes a failure, or `clap`'s verdict, where it belongs and answers with its code.
///
/// The program name and the numbers are the application's; the mapping is the
/// library's [`clifacet::report::render`], so help lands on the data stream, a usage
/// error and a failure on the diagnostic stream, each costing what [`EXIT`] says.
pub(crate) fn report<'a>(
    report: impl Into<clifacet::report::Report<'a>>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    clifacet::report::render("example", report, &EXIT, out, err)
}
