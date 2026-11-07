//! The entry points, and the flag and prompt path: parse, settle the mode, collect, run,
//! report.
//!
//! The entry points come in three depths — real process, explicit capabilities and
//! frontend, and any command with this input — so a test can drive exactly the code a
//! real invocation takes, with buffers instead of a terminal.

use std::io::Write;

use clap::Parser;

use clifacet::policy::{Capabilities, Mode};
use clifacet::prompt::{Collect, Dialog, Prompter, Session, Terminal};
use clifacet::{Command, Facet, Failure};

use crate::commands::{Cli, Example, ExampleInput, ExampleOutput};
use crate::exit::{EXIT_SUCCESS, report};
use crate::interaction::{POLICY, request};

/// Runs one invocation, writing to the given streams and returning its exit code.
///
/// Data goes to `out` only, diagnostics to `err` only. Prompts, when the mode
/// permits them, come from the process's terminal: this is the entry point that
/// owns real streams, and it is deliberately thin so that everything below it can
/// be driven by buffers.
pub fn run(cli: Cli, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    run_with(cli, Capabilities::detected(), dialog_frontend, out, err)
}

/// The frontend a real process uses: the library's `inquire` dialog.
///
/// Presentation is a choice the application makes here and nowhere else. The dialog
/// asks the same two questions [`terminal_frontend`] does — a value and a yes/no —
/// and differs only in how they look, which is why swapping it changed no other line
/// in this crate: the session still decides *whether* a question may be asked, and a
/// supplied value is still never re-asked.
///
/// Constructing it touches no terminal; the dialog is only entered when a question is
/// actually asked, so a plan that turns out to ask nothing costs nothing.
#[must_use]
pub fn dialog_frontend() -> Box<dyn Prompter> {
    Box::new(Dialog::new())
}

/// The dependency-free reader: the same seam, without a dialog.
///
/// It is what a scripted flow, a test or a terminal without a rich frontend wants —
/// a question written to `stderr` and a line read from `stdin` — and it is kept here
/// to show that the choice is the application's: both implement [`Prompter`], and
/// [`run_with`] takes either.
///
/// Constructing it locks stdin and holds stderr, but touches neither until a
/// question is actually asked, so this is safe to build and discard.
#[must_use]
pub fn terminal_frontend() -> Box<dyn Prompter> {
    Box::new(Terminal::stdio())
}

/// Runs one invocation against explicit capabilities and an explicit frontend.
///
/// This is the seam the tests use: capabilities decide whether prompting is
/// possible at all, and the frontend builder is called only when the resolved plan
/// asks the user something. A scripted frontend therefore exercises exactly the
/// code a terminal frontend would.
pub fn run_with(
    cli: Cli,
    capabilities: Capabilities,
    build_frontend: impl FnOnce() -> Box<dyn Prompter>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    run_with_command(&Example, cli, capabilities, build_frontend, out, err)
}

/// Runs one validated invocation of any command that accepts this example's input.
///
/// The example's own [`Example`] is the ordinary case; a different command with the
/// same input and output types is how a caller observes what the run did — see
/// [`clifacet::harness::Recording`]. Because the input type is fixed, "a different
/// command" cannot quietly be a different pipeline: it is the same parse, the same
/// validation and the same stream policy.
pub fn run_with_command<C>(
    command: &C,
    cli: Cli,
    capabilities: Capabilities,
    build_frontend: impl FnOnce() -> Box<dyn Prompter>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32
where
    C: Command<Input = ExampleInput, Output = ExampleOutput>,
{
    let plan = match POLICY.resolve(request(cli.interaction), capabilities) {
        Ok(plan) => plan,
        Err(refusal) => {
            let failure = Failure::validation(refusal.message());
            return report(&failure, out, err);
        }
    };

    // The full-screen mode is hosted here rather than only in `run_tui`, so a
    // `--tui` run takes the same parse, validation and reporting path as the rest.
    if plan.mode() == Mode::Tui {
        // The screen is drawn on the diagnostic stream the caller passed, like every
        // other thing a person sees. Drawing it on the data stream would put escape
        // sequences in what a caller asked for, which is the one thing the stream
        // contract forbids — and reaching for the process's own stderr would bypass
        // the streams this function was given.
        return crate::form::run_tui_on(command, err, clifacet::tui::live_keys(), out);
    }

    let mut session = Session::start(plan, build_frontend);
    match session.run(command, move |session| collect(session, cli)) {
        Ok(output) => match output.write_to(out) {
            Ok(()) => EXIT_SUCCESS,
            Err(error) => {
                let failure = Failure::execution(format!("cannot write the output: {error}"));
                report(&failure, out, err)
            }
        },
        Err(failure) => report(&failure, out, err),
    }
}

/// Asks what the declarations ask, then validates through the same bridge the flags use.
///
/// Both halves are generated from [`Cli`]'s fields: [`Collect`] asks for a value no
/// flag supplied and confirms `--explode`, each only when the plan permits, and
/// [`Facet::validate`] is the [`TryFrom<Cli>`] bridge. A value the user supplied is
/// never re-asked — which is what makes a prompted run and a flag run the same
/// command rather than two behaviours.
fn collect(session: &mut Session, cli: Cli) -> Result<ExampleInput, Failure> {
    cli.collect(session).and_then(Facet::validate)
}

/// Parses process arguments and runs them.
///
/// This is the entry point a real binary uses: it detects capabilities and prompts
/// on the process's terminal, and it is split from `main` so tests can pass their
/// own arguments and buffers instead of a terminal and a process status.
pub fn main(
    argv: impl IntoIterator<Item = std::ffi::OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match Cli::try_parse_from(argv) {
        Ok(cli) => run(cli, out, err),
        Err(error) => report(&error, out, err),
    }
}

/// The scriptable entry point: explicit capabilities and frontend, no detection.
///
/// Tests and scripted flows use this, so nothing in a test can reach a real
/// terminal by accident — a question without an answer fails instead of hanging.
pub fn main_with(
    argv: impl IntoIterator<Item = String>,
    capabilities: Capabilities,
    build_frontend: impl FnOnce() -> Box<dyn Prompter>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    main_with_command(&Example, argv, capabilities, build_frontend, out, err)
}

/// Parses `argv`, then runs it against `command`.
///
/// The whole path a real invocation takes — `clap` first, usage errors included — so a
/// parity check compares entry points rather than pieces.
pub fn main_with_command<C>(
    command: &C,
    argv: impl IntoIterator<Item = String>,
    capabilities: Capabilities,
    build_frontend: impl FnOnce() -> Box<dyn Prompter>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32
where
    C: Command<Input = ExampleInput, Output = ExampleOutput>,
{
    match Cli::try_parse_from(argv) {
        Ok(cli) => run_with_command(command, cli, capabilities, build_frontend, out, err),
        Err(error) => report(&error, out, err),
    }
}
