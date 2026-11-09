//! A friendly wrapper around the Backlog.md CLI, built on the published API.
//!
//! The point of this crate is not to reimplement Backlog.md. It is to show what a *thin*
//! layer over somebody else's CLI costs when the layer is built on clifacet: typed input
//! ([`Request`]) instead of a stringly argument soup, a decision that is data
//! ([`Invocation`]) instead of a side effect, and one planned command, so `--dry-run`
//! prints the line a real run executes.
//!
//! ```text
//! bl                          # list, the thing you do most
//! bl --json show CLI-20       # one --json, not one per subcommand
//! bl new "Ship it" --ac one   # the title is optional; you are asked
//! bl done CLI-20              # confirmed, and --yes skips the question
//! bl --dry-run done CLI-20    # prints the exact command instead of running it
//! ```
//!
//! # What the application itself writes
//!
//! Three things, and this file is all of them:
//!
//! - the context — the [`Program`] to forward to, built once per run;
//! - the [`Handler`](clifacet::Handler) that turns a request into a plan;
//! - [`run_app`], which is the whole of `main` now that clifacet's [`App`] owns the
//!   pipeline (CLI-46, CLI-48).
//!
//! Everything else is declared: [`Cli`], [`Commands`] and [`Request`] come from the one
//! `#[clifacet::registry]` line on `mod commands`, and each leaf is one file.
//!
//! # Layout
//!
//! ```text
//! src/main.rs           one call: run the application
//! src/lib.rs            this file: the command, the wiring, and `run_app`
//! src/commands/mod.rs   the folder's own types: `Request`, `From<Input>`, `forward`
//! src/commands/<leaf>.rs  one command: its flags, its input, its questions, its argv
//! src/commands/_task.rs what show, start and done share: a task ID and moving it
//! src/status.rs         the `Status` every spelling of a status parses to
//! src/program.rs        the program forwarded to, and the one place that spawns it
//! src/invocation.rs     a request as a command line, purely
//! ```

/// The command folder: seven leaves, their tree, and the root the runner parses.
///
/// This line is the whole registration — the module list, the `Commands` tree, the `Action`
/// request, the `Input` and the `Cli` root, from the build script's report. `mod.rs` keeps
/// what is the folder's own: the root's `json` flag, and the conversions into `Request`.
///
/// The application's context needs no line here. Each command that needs the program takes
/// it as a `&Program` parameter, so the type is written where it is used and the tree threads
/// it to whichever leaf was chosen.
#[clifacet::registry(
    root = { "json": bool },
    request = Action,
    name = "bl",
    about = "A friendlier front door to the Backlog.md CLI",
    long_about = "A friendlier front door to the Backlog.md CLI.\n\n\
                  Everything is forwarded to `backlog`; nothing is reimplemented. \
                  What this adds is the three things everyone retypes: a status \
                  shortcut, one --json for the whole run, and --dry-run."
)]
mod commands;
mod invocation;
mod program;
mod status;

use std::convert::Infallible;
use std::process::ExitCode;

use clifacet::command::{Plan, PlannedWith};
use clifacet::contract::ExitPolicy;
use clifacet::failure::Failure;
use clifacet::{App, Globals};

pub use commands::{
    Action, Cli, Commands, Filter, Input, NewTask, Query, Request, TaskRef, collect_new,
    confirm_status, task_id,
};
pub use invocation::{Invocation, invocation_of};
pub use program::{Program, run};
pub use status::Status;

/// The global options every application gets, re-exported so this one can name them.
pub type GlobalOptions = Globals;

/// The exit codes this application gives its outcomes.
pub const EXIT: ExitPolicy = ExitPolicy::standard();

/// The application, as the runner's command: the root's input, planned into what it runs.
///
/// A unit struct, because it holds nothing: the program it forwards to is the context,
/// built once per run by the closure [`app`] hands the runner.
#[derive(Debug, Clone, Copy, Default)]
pub struct Backlog;

/// Request to plan, and the plan to execute: one path, so a preview is the run described.
///
/// The input is the root's, `command` and `json` together, which is the application's own
/// decision: a bare `bl` means a list, and the root's flag travels with whatever was chosen.
/// The plan's steps are the sentence and the runnable line `--dry-run` prints; its work is
/// [`run`]. Because clifacet implements [`Handler`](clifacet::Handler) for a `PlannedWith`
/// command as "plan, then execute", there is no other way to run one.
#[clifacet::planned]
impl PlannedWith<Program> for Backlog {
    type Input = Input;
    /// The wrapped tool's own exit status, as the number the plan carries.
    type Output = String;

    fn plan<'a>(&'a self, input: Input, program: &'a Program) -> Result<Plan<'a, String>, Failure> {
        let invocation = invocation_of(input.into(), program);
        let steps = [
            invocation.explain().to_owned(),
            format!("    {}", invocation.shell_line()),
        ];
        let plan = Plan::new(move || run(invocation.argv()).map(|status| status.to_string()));
        Ok(steps.into_iter().fold(plan, Plan::step))
    }
}

/// The application, configured and ready to run.
///
/// `build` is how the program arrives. The binary passes [`Program::default`], a test its
/// own fake; nothing is read from the environment, so two tests cannot see each other's
/// dependency.
#[must_use]
pub fn app(
    build: impl Fn() -> Result<Program, Infallible> + 'static,
) -> App<'static, Cli, Backlog, clifacet::Plain, Program> {
    App::<Cli>::new().command(&Backlog).context(build)
}

/// Runs one invocation from the process's own command line: the whole of `main`.
#[must_use]
pub fn run_app() -> ExitCode {
    app(|| Ok(Program::default())).run()
}

/// The request a parsed command line stands for: the chosen command and the root's flag.
///
/// Kept as a free function beside [`Input`]'s `From` impl, because a test that asserts a
/// command line reads better naming the request it stands for than converting to it.
#[must_use]
pub fn request_of(input: Input) -> Request {
    input.into()
}
