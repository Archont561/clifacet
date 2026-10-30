//! Composable command-line presentation modes.
//!
//! Clifacet keeps one UI-free command core and renders it through flag-based,
//! interactive and TUI frontends (decision-1). This release is the first slice of
//! that architecture: a command seam ([`Command`]), a library-owned failure type
//! ([`Failure`]), and a worked, tested reference application ([`example`]).
//!
//! # The seam
//!
//! A command takes validated input and returns data or a failure. It never writes
//! a stream and never reads a terminal, so the same value serves every frontend:
//!
//! ```no_run
//! use clifacet::{Command, Failure};
//!
//! struct Count;
//!
//! impl Command for Count {
//!     type Input = usize;
//!     type Output = usize;
//!
//!     fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
//!         Ok(input * 2)
//!     }
//! }
//! ```
//!
//! The application owns everything around the seam: parsing with `clap`,
//! validation, which stream the data goes to, and which exit code a failure
//! costs. [`example`] shows that division end to end.
//!
//! # Not implemented yet
//!
//! - Interactive and TUI frontends: the `interactive` and `tui` features reserve
//!   their dependency boundaries only.
//! - JSON output: the `json` feature reserves `serde_json`; no flag selects it yet.
//! - Runtime mode resolution, prompts and cancellation: M2. Cancellation will need
//!   its own failure category, so [`FailureKind`] is `#[non_exhaustive]`.
//! - The companion derive that builds the `clap` surface and the input conversion
//!   is M1's next task; [`example`] writes that conversion by hand today.

#![allow(
    clippy::module_name_repetitions,
    reason = "`failure::Failure` and `command::Command` are the names the architecture \
              documents; qualifying them twice would read worse at every call site"
)]

// The derive emits `::clifacet::…` paths, and this crate is its own dependency
// from the macro's point of view. This alias is the documented way to make that
// resolve, and it is why `#[derive(Facet)]` works inside this crate too.
extern crate self as clifacet;

pub mod command;
pub mod example;
pub mod facet;
pub mod failure;
pub mod policy;

/// The version of the `clifacet` package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of the `clifacet` package.
#[must_use]
pub const fn version() -> &'static str {
    VERSION
}

pub use command::Command;
pub use facet::Facet;
pub use failure::{Failure, FailureKind};

/// `clap`, re-exported because [`example::Cli`] implements its traits.
///
/// A caller that parses with [`example::Cli`] needs `clap::Parser` in scope, so
/// the version they use has to be this one: re-exporting it removes the chance of
/// resolving a different `clap` than the derive was built against.
pub use clap;

/// Expands to the version of the `clifacet-macros` package.
pub use clifacet_macros::version;

/// Derives [`Facet`], the companion derive for a clap command.
///
/// Re-exported so an application depends on `clifacet` alone; the trait and the
/// macro share the name `Facet` the way `serde`'s `Serialize` does.
pub use clifacet_macros::Facet;

#[cfg(test)]
mod tests;
