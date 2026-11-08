//! An application whose commands are discovered from its folder (CLI-50).
//!
//! `src/commands/mod.rs` holds `clifacet::commands!()` with no list, and `build.rs` scans the
//! folder. Dropping a file into `src/commands/` is the whole of adding a command — no list to
//! edit, no `mod` line to write, no variant to add.
//!
//! The example is deliberately small: it is a proof of the discovery mechanism and of its
//! error messages, not another full application. `examples/simple` is the application.

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
/// The tree's output is [`Said`] and its leaves are planned, so the runner previews a
/// `--dry-run` through the leaf that would run instead of refusing.
#[must_use]
pub fn run() -> std::process::ExitCode {
    clifacet::run::<Cli>()
}
