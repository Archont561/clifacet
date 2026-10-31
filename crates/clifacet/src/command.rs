//! The UI-free command seam.
//!
//! A [`Command`] receives validated input and answers with data or a
//! [`Failure`](crate::Failure). It takes no stream, reads no terminal and holds
//! no presentation state, which is what lets the same command serve flag-based,
//! interactive and TUI frontends (decision-1) without knowing any of them exist.
//!
//! Parsing, validation and rendering stay outside this trait. The application
//! turns its `clap` arguments into `Input`, rejects bad values as
//! [`Failure::validation`](crate::Failure::validation), and writes whatever the
//! command returns. See the `clifacet-example` crate for a complete, tested example of that
//! division.

use crate::failure::Failure;

/// A command that runs without a terminal, a prompt or a user interface.
///
/// # Example
///
/// ```
/// use clifacet::{Command, Failure};
///
/// struct Echo;
///
/// impl Command for Echo {
///     type Input = String;
///     type Output = String;
///
///     fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
///         if input.is_empty() {
///             return Err(Failure::execution("nothing to echo"));
///         }
///         Ok(input)
///     }
/// }
///
/// assert_eq!(Echo.execute("hi".to_owned()).unwrap(), "hi");
/// assert_eq!(
///     Echo.execute(String::new()).unwrap_err().kind(),
///     clifacet::FailureKind::Execution,
/// );
/// ```
pub trait Command {
    /// Input the command accepts. Construct it after parsing and validation, so
    /// this type can be total: every value of it is runnable.
    type Input;

    /// Data the application may render or print.
    ///
    /// This is the machine data a caller asked for. It is not a message: the
    /// application decides which stream it belongs on and how it is formatted.
    type Output;

    /// Runs the command against validated input.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::execution`] when the operation cannot complete. Input
    /// problems are rejected before this call, as [`Failure::validation`], by
    /// whoever built the input.
    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure>;
}
