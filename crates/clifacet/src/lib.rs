//! Composable command-line presentation modes.
//!
//! Clifacet keeps one UI-free command core and renders it through flag-based,
//! interactive and TUI frontends (decision-1). This release is the first slice of
//! that architecture: a command seam ([`Command`]), a library-owned failure type
//! ([`Failure`]), and the frontends above them.
//!
//! The worked reference application lives beside this crate, in
//! `crates/clifacet-example`: it depends on `clifacet` and on nothing private, so
//! every seam it uses is a seam a caller has.
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
//! Everything around the seam — parsing with `clap`, asking, validation, which stream
//! the data goes to, and which exit code a failure costs — is one pipeline, and
//! [`app`] runs it: `main` is `clifacet::run::<Cli>()`, or an [`App`] when an
//! application changes a default. `clifacet-example` shows that division end to end.
//!
//! # Four frontends, one command
//!
//! Parsing stays the application's: `clifacet-example` shows a `clap` command, its
//! validated input ([`Facet`]), and the [`policy`] that decides how interactive a
//! run may be. Everything after that is shared:
//!
//! - flags — the command line the user typed;
//! - [`prompt`] — questions asked only for what the flags left out;
//! - [`tui`] — a full-screen form built from public components (`tui` feature);
//! - [`harness`] — the public way to prove the mode changed nothing else.
//!
//! [`Globals`] gives every application `--yes` and `--dry-run`; a dry run prints what a
//! [`command::Planned`] command would do, through [`Command::preview`].
//!
//! [`contract`] states what a run may write where — data on the data stream, everything
//! a person sees on the diagnostic one — and maps each outcome to an exit code;
//! [`report`] applies that mapping, writing a failure or `clap`'s verdict where it
//! belongs and answering with what it costs.
//!
//! [`secret`] wraps a credential so nothing the framework prints can show it, and
//! [`provenance`] records where each value came from without recording the value.
//!
//! # Features
//!
//! - `interactive` — the `inquire` dialog behind [`prompt::Prompter`];
//! - `tui` — the full-screen frontend in [`tui`];
//! - `serde` — `Serialize` for [`Failure`] and [`FailureKind`] (a kind and a message)
//!   and for [`secret::Secret`], which serialises redacted. The library writes no JSON
//!   itself: output is the application's, so a JSON failure body is the application
//!   serialising a [`Failure`] under its own flag.

#![allow(
    clippy::module_name_repetitions,
    reason = "`failure::Failure` and `command::Command` are the names the architecture \
              documents; qualifying them twice would read worse at every call site"
)]

// The derive emits `::clifacet::…` paths, and this crate is its own dependency
// from the macro's point of view. This alias is the documented way to make that
// resolve, and it is why `#[derive(Facet)]` works inside this crate too.
extern crate self as clifacet;

pub mod app;
pub mod command;
pub mod contract;
pub mod facet;
pub mod failure;
pub mod harness;
pub mod policy;
pub mod prompt;
pub mod provenance;
pub mod report;
pub mod secret;

/// The full-screen frontend, available with the `tui` feature.
#[cfg(feature = "tui")]
pub mod tui;

/// The version of the `clifacet` package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of the `clifacet` package.
#[must_use]
pub const fn version() -> &'static str {
    VERSION
}

pub use app::{App, Globals, Plain, Render, run};
pub use command::Command;
pub use facet::Facet;
pub use failure::{Failure, FailureKind};

/// `clap`, re-exported so a caller can name the version this crate was built against.
///
/// A caller that parses with a derived command needs `clap::Parser` in scope, so the
/// version they use has to be this one: re-exporting it removes the chance of
/// resolving a different `clap` than the derive was built against.
pub use clap;

/// Expands to the version of the `clifacet-macros` package.
pub use clifacet_macros::version;

/// Wraps a struct in clap's parser and the [`Facet`] bridge, in one attribute.
///
/// This is the same pairing as `#[derive(clap::Parser, clifacet::Facet)]` plus
/// `#[facet(input = …)]`, emitted as one attribute because the two have to agree:
///
/// ```
/// use clifacet::{Facet, Failure};
///
/// #[clifacet::command(input = Greeting)]
/// #[command(name = "greet", about = "Say hello")]
/// struct Cli {
///     #[arg(long, alias = "nickname")]
///     name: Option<String>,
/// }
///
/// #[derive(Debug, PartialEq)]
/// struct Greeting {
///     name: Option<String>,
/// }
///
/// let cli = <Cli as clifacet::clap::Parser>::try_parse_from(["greet", "--nickname", "ada"])
///     .expect("an alias parses");
/// assert_eq!(cli.validate().expect("valid").name.as_deref(), Some("ada"));
/// ```
///
/// It takes the same keys as `#[facet(…)]`. `clap` must still be a direct dependency of
/// the calling crate, because clap's own derive expands to code naming `clap::…`; this
/// crate re-exports [`clap`], so the version is the one this macro was built against.
///
/// The name lives in the macro namespace, which is why it can sit beside the [`command`]
/// module without either shadowing the other.
pub use clifacet_macros::command;

/// Declares a command tree where its commands live (decision-14).
///
/// The enum stays clap's `Subcommand`, one variant per subcommand and one leaf struct per
/// variant; the attribute generates the conversion to the application's `Request`, the
/// collection, and — with `output = …` — the dispatch to the chosen leaf. Validation
/// consumes the parsed command, so a tree that runs keeps a clone of itself to run with:
///
/// ```
/// use clifacet::{Command, Facet, Failure};
///
/// #[clifacet::command(input = Input)]
/// #[command(name = "todo")]
/// struct Cli {
///     #[clifacet(subcommands)]
///     command: Option<Commands>,
/// }
///
/// struct Input {
///     command: Option<Request>,
/// }
///
/// #[clifacet::subcommands(input = Request, output = String)]
/// #[derive(Clone)]
/// enum Commands {
///     /// Add a task.
///     Add(Add),
///     /// Mark a task done.
///     #[clifacet(alias = "finish")]
///     Done(Done),
/// }
///
/// enum Request {
///     Add(AddInput),
///     Done(DoneInput),
/// }
///
/// #[clifacet::command(input = AddInput)]
/// #[derive(Clone)]
/// struct Add {
///     #[clifacet(trim, non_empty)]
///     title: String,
/// }
///
/// struct AddInput {
///     title: String,
/// }
///
/// #[clifacet::command(input = DoneInput)]
/// #[derive(Clone)]
/// struct Done {
///     id: u32,
/// }
///
/// struct DoneInput {
///     id: u32,
/// }
///
/// impl Command for Add {
///     type Input = AddInput;
///     type Output = String;
///
///     fn execute(&self, input: AddInput) -> Result<String, Failure> {
///         Ok(format!("added {}", input.title))
///     }
/// }
///
/// impl Command for Done {
///     type Input = DoneInput;
///     type Output = String;
///
///     fn execute(&self, input: DoneInput) -> Result<String, Failure> {
///         Ok(format!("finished #{}", input.id))
///     }
/// }
///
/// let cli = <Cli as clifacet::clap::Parser>::try_parse_from(["todo", "finish", "7"]).unwrap();
/// let tree = cli.command.clone().expect("a subcommand was given");
/// let request = cli.validate().unwrap().command.expect("so it converted");
///
/// assert_eq!(tree.execute(request).unwrap(), "finished #7");
/// ```
pub use clifacet_macros::subcommands;

/// Derives [`Facet`], the companion derive for a clap command.
///
/// Re-exported so an application depends on `clifacet` alone; the trait and the
/// macro share the name `Facet` the way `serde`'s `Serialize` does.
pub use clifacet_macros::Facet;

#[cfg(test)]
mod tests;
