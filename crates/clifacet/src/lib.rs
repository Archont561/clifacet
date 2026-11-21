//! Build one command-line command; run it in any of three terminal modes.
//!
//! Clifacet keeps one UI-free command core and runs it in three terminal modes:
//! noninteractive, interactive and TUI (decision-1). This release is the first slice
//! of that architecture: a command seam ([`Command`]), a library-owned failure type
//! ([`Failure`]), and the modes above them.
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
pub mod cli;
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
pub mod settings;

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

/// Registers the commands of one folder in one line (decision-19 item 3, CLI-33).
///
/// ```ignore
/// clifacet::commands!(list, show, new);
/// clifacet::commands!(list, show; globals = Options, output = Report, context = Store);
/// ```
///
/// Each module named holds one command — the file stem is its name in kebab case, the
/// struct is that stem in Pascal case — and this call is the only place it is registered.
/// It expands to the modules and their re-exports, `Commands` (the tree), `Request` (what
/// the user asked for, validated), `Input` and `Cli` (the root, with `Globals` flattened).
///
/// The list is the whole registry, and the compiler checks it — a `Request` variant left out
/// is a compile error, a module listed twice is one too. The tree is `#[clifacet::subcommands]`,
/// so dispatch is compile-time: no subcommand reaches a command as an untyped string.
///
/// Note: Legacy `build.rs` directory scanning has been retired (decision-25, CLI-71).
/// For declarative controllers, use [`#[clifacet::app]`](macro@app) and
/// [`#[clifacet::commands]`](macro@commands).
///
/// The keys after the module names declare what the folder's commands share:
///
/// - `globals = Type` — the application's own global options, a `clap::Args` type,
///   flattened beside [`Globals`] and kept out of `Input`;
/// - `output = Type` — what every leaf's output converts into, so the tree is a
///   [`Command`] (or a [`Handler`] of `context`) and the runner can run it;
/// - `context = Type` — the application's context, threaded to the chosen leaf;
/// - `planned` — the tree delegates `plan`, so a dry run sees each leaf's own plan;
/// - `name = "…"`, `about = "…"` — the root's name and description, otherwise Cargo's;
/// - `version_off` — no `--version`.
///
/// `$crate` is used for every path it emits, so a caller needs only `clifacet` and `clap`.
/// The work is done by a proc macro (`#[doc(hidden)]`), because a `macro_rules!` cannot
/// write an attribute that carries a type it was handed.
pub use clifacet_macros::commands;

/// The proc macro behind [`commands!`]; hidden because calling it directly skips `$crate`.
#[doc(hidden)]
pub use clifacet_macros::__commands;

/// Registers a folder of commands on the module that holds it (CLI-58).
///
/// ```ignore
/// // src/lib.rs
/// #[clifacet::registry]
/// mod commands {}
/// ```
///
/// The same registration as [`commands!`], written where the folder is declared rather than
/// inside it: the module list comes from the folder's build report (so this needs the same
/// `build.rs` call and `build` feature discovery does), and the folder's `mod.rs` keeps
/// everything that is not registration — its `Request`, its helpers, its `impl`s — with no
/// `commands!(..)` call left in it.
///
/// ```ignore
/// #[clifacet::registry(output = crate::Report, planned)]
/// mod commands {}
/// ```
///
/// The braces are not decoration: an *outlined* module (`mod commands;`) in an attribute's
/// input is unstable below Rust 1.99, and this crate supports older compilers than that. An
/// empty body is filled from the folder's `mod.rs`; a body written here is the folder itself,
/// and is kept as written beside the registration.
///
/// A context is not declared here. A command that needs one takes it as a `cx: &Cx`
/// parameter, so the type is written once, where it is used, and the tree threads it to
/// whichever leaf is chosen.
///
/// # What the folder may not start with
///
/// The folder's `mod.rs` is pulled in with an `include!`, which cannot carry inner attributes
/// or `//!` docs. A file that begins with them is refused with a message saying to move the
/// documentation onto this line and to use `//` comments inside the folder — rather than
/// Rust's own message, which is about the fragment and never names the attribute.
///
/// # Keys
///
/// `output = Type`, `globals = Type`, `planned`, `version_off`, `name = "…"`,
/// `about = "…"`, `long_about = "…"`, `root = { "field": Type }`, and `path = "…"` for a
/// folder whose name does not say where it is. There is no module list — the folder is the
/// list — and the generated enum is `Action` unless `request = Ident` names it, so a folder
/// may keep a hand-written `Request` of its own beside it.
pub use clifacet_macros::registry;

pub use app::{App, Globals, JsonRenderer, Plain, Render, run, run_cli};
pub use cli::{
    ActionLog, Cli, ContextCell, FromCli, IntoFailure, IntoOutcome, Json, OutcomePayload,
    PlannedOutcome, Structured,
};
pub use command::{Command, Handler};
pub use facet::Facet;
pub use failure::{Failure, FailureKind};
pub use report::{ErrorDetail, ErrorPayload, render_json};

#[cfg(feature = "schema")]
pub mod schema;
#[cfg(feature = "schema")]
pub use schema::{ArgumentMetadata, CliMetadata};

#[cfg(feature = "skill")]
pub mod skill;
#[cfg(feature = "skill-minijinja")]
pub use skill::{DEFAULT_SKILL_TEMPLATE, MiniJinjaSkillRenderer};
#[cfg(feature = "skill")]
pub use skill::{SkillProvider, SkillRenderer, WriteSkillOptions, execute_write_skill};

#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "mcp")]
pub use mcp::{
    McpServer, arguments_to_argv, generate_tools_list, run_server_streams, run_stdio_server,
};

#[cfg(feature = "completions")]
pub mod completions;
#[cfg(feature = "completions")]
pub use completions::Shell;

#[cfg(feature = "async")]
pub mod r#async;
#[cfg(feature = "async")]
pub use r#async::{
    AsyncCommand, AsyncHandler, AsyncPlan, AsyncPlanned, AsyncPlannedWith, BoxFuture, Cancel,
    CancelledFuture, LocalAsyncHandler, NoopProgressSink, ProgressEvent, ProgressSink,
    RecordingProgressSink, RunCx,
};

/// `clap`, re-exported so a caller can name the version this crate was built against.
///
/// A caller that parses with a derived command needs `clap::Parser` in scope, so the
/// version they use has to be this one: re-exporting it removes the chance of
/// resolving a different `clap` than the derive was built against.
pub use clap;

/// Expands to the version of the `clifacet-macros` package.
pub use clifacet_macros::version;

pub use clifacet_macros::app;
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
/// It takes the same keys as `#[facet(…)]`. This crate re-exports [`clap`] as
/// `clifacet::clap`, and all emitted derives route through it.
///
/// The name lives in the macro namespace, which is why it can sit beside the [`command`]
/// module without either shadowing the other.
pub use clifacet_macros::command;
pub use clifacet_macros::group;

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

/// Makes a [`PlannedWith`](command::PlannedWith) impl a [`Handler`] that runs through its plan.
///
/// The bridge a blanket impl cannot provide — it would overlap the one every [`Command`]
/// has — written beside the impl it comes from: `handle` is `plan(input, cx)?.execute()`
/// and `preview` is the plan, so `--dry-run` prints what a real run executes. See
/// [`PlannedWith`](command::PlannedWith) for an example.
pub use clifacet_macros::planned;

/// Derives [`Facet`], the companion derive for a clap command.
///
/// Re-exported so an application depends on `clifacet` alone; the trait and the
/// macro share the name `Facet` the way `serde`'s `Serialize` does.
pub use clifacet_macros::Facet;

#[cfg(test)]
mod tests;
