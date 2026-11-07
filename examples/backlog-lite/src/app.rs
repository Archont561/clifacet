//! The application layer: settle the mode, collect, plan, then print or run.
//!
//! This is the only module that sees streams and exit codes. The commands below it
//! produce values; here a value becomes either a `--dry-run` line or a process.

use std::io::Write;

use clap::Parser;
use clifacet::Facet;
use clifacet::command::{Plan, Planned};
use clifacet::contract::ExitPolicy;
use clifacet::failure::Failure;
use clifacet::policy::{Capabilities, Mode, Policy, Request as ModeRequest};
use clifacet::prompt::{Collect, Prompter, Session, Terminal};

use crate::commands::{Cli, Request};
use crate::invocation::{invocation_of, run};

/// The exit codes this application gives its outcomes.
pub const EXIT: ExitPolicy = ExitPolicy::standard();

/// Runs one invocation and answers with the status the process should exit with.
///
/// Capabilities and the frontend are parameters rather than lookups, so a test can
/// run the whole application — including the questions — without a terminal.
pub fn app(
    cli: Cli,
    capabilities: Capabilities,
    build_frontend: impl FnOnce() -> Box<dyn Prompter>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let requested = if capabilities.supports(Mode::Interactive) {
        ModeRequest::mode(Mode::Interactive)
    } else {
        ModeRequest::none()
    };
    let plan = match Policy::up_to(Mode::Interactive)
        .resolve(requested.assuming_yes(cli.yes), capabilities)
    {
        Ok(plan) => plan,
        Err(refusal) => {
            report(err, &refusal.message());
            return EXIT.execution();
        }
    };
    let dry_run = cli.dry_run;
    let mut session = Session::start(plan, build_frontend);
    // The chosen leaf asks its own questions, then validates by its own rules; an
    // absent subcommand asks nothing and reaches `invocation_of` as `None`.
    let planned = cli
        .collect(&mut session)
        .and_then(Facet::validate)
        .and_then(|request| Backlog.plan(request));
    // The dry run describes the plan and the real run executes it: one value, so the
    // printed line is the line that runs.
    let outcome = match planned {
        Ok(plan) if dry_run => {
            report(out, &plan.describe());
            return EXIT.success();
        }
        Ok(plan) => plan.execute(),
        Err(failure) => Err(failure),
    };
    match outcome {
        Ok(code) => code,
        Err(failure) => clifacet::report::render("bl", &failure, &EXIT, out, err),
    }
}

/// The wrapper as a clifacet command: a request is planned as one invocation of the
/// wrapped tool, and running it is executing that plan.
///
/// The plan's steps are the sentence and the runnable line `--dry-run` prints; its work
/// is [`run`]. Because the library implements [`clifacet::Command`] for a [`Planned`]
/// command as "plan, then execute", there is no other way to run one.
#[derive(Debug, Clone, Copy, Default)]
pub struct Backlog;

impl Planned for Backlog {
    type Input = Request;
    /// The wrapped tool's own exit status, passed through unchanged.
    type Output = i32;

    fn plan(&self, request: Request) -> Result<Plan<'_, i32>, Failure> {
        let invocation = invocation_of(request);
        let steps = [
            invocation.explain().to_owned(),
            format!("    {}", invocation.shell_line()),
        ];
        let plan = Plan::new(move || run(&invocation));
        Ok(steps.into_iter().fold(plan, Plan::step))
    }
}

/// Writes one line, ignoring a closed stream: a broken pipe is not a failure to report.
fn report(stream: &mut dyn Write, line: &str) {
    let _ = writeln!(stream, "{line}");
}

/// The frontend a real process uses: the library's dialog, on the process terminal.
#[must_use]
pub fn terminal_frontend() -> Box<dyn Prompter> {
    Box::new(Terminal::stdio())
}

/// The entry point a real binary calls.
#[must_use]
pub fn main(
    argv: impl IntoIterator<Item = std::ffi::OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match Cli::try_parse_from(argv) {
        Ok(cli) => app(cli, Capabilities::detected(), terminal_frontend, out, err),
        // Help is the result the user asked for, on the data stream; a usage error is
        // a diagnostic. `render` applies that, with the codes `EXIT` gives them.
        Err(error) => clifacet::report::render("bl", &error, &EXIT, out, err),
    }
}
