//! An application whose commands are organized one file per command (CLI-50, CLI-71).
//!
//! Declarative controller and command trees compose without build scripts (decision-25).

pub mod commands;

pub use commands::{Cli, Commands, Input, Request};

/// What every command in this example answers: the words it was given.
///
/// A discovered tree needs one output type, because `output = ..` is a property of the folder
/// rather than of a leaf. This is that type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said(pub String);

impl From<String> for Said {
    fn from(said: String) -> Self {
        Self(said)
    }
}

impl std::fmt::Display for Said {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Runs the application: the one call every clifacet `main` is.
///
/// The tree's output is [`Said`], so the runner has one type to render.
#[must_use]
pub fn run() -> std::process::ExitCode {
    clifacet::run::<Cli>()
}
