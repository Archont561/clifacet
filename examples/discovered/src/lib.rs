//! An application whose commands are discovered from its folder (CLI-50).
//!
//! `#[clifacet::registry]` on the module below is the whole registration, and `build.rs` scans
//! the folder. Dropping a file into `src/commands/` is the whole of adding a command — no list
//! to edit, no `mod` line to write, no variant to add.
//!
//! The example is deliberately small: it is a proof of the discovery mechanism and of its
//! error messages, not another full application. `examples/simple` is the application.

/// The commands of this application, discovered from this folder (CLI-50).
///
/// This line is the whole registration. `build.rs` scanned `src/commands/`; this attribute
/// writes the `mod` lines, the `Commands` tree, the `Request`, the `Input` and the `Cli` root
/// from that report, and pulls the folder's `mod.rs` in beside them, so the folder holds
/// everything that is not registration — its helper, its re-exports — and no `commands!(..)`
/// call at all. Dropping a file into `src/commands/` remains the whole of adding a command.
///
/// The folder's file is included, which is what makes this work and what it costs: a file
/// pulled in that way may not begin with `//!`, so the folder documents itself with `//`.
///
/// `mod commands {}` rather than `mod commands;`: an *outlined* module in an attribute's
/// input is unstable below Rust 1.99, and this crate's floor is lower than that, so the
/// braces are what make the declaration legal on every supported compiler. The body stays
/// empty — `include!` fills it from the folder — and writing items here would replace the
/// folder instead of adding to it.
#[clifacet::registry(
    output = crate::Said,
    request = Request,
    name = "discovered",
    about = "A proof that a folder registers itself"
)]
pub mod commands {}

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
