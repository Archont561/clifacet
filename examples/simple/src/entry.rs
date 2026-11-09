//! The entry points: the application as one [`App`], and the depths a test drives it at.
//!
//! [`app`] is the whole application — the command, the mode policy and the exit codes —
//! and `main.rs` runs it. The functions below are the same runner with less taken from
//! the process: explicit capabilities and frontend, an already-parsed command line, or
//! another command with this input. Every one of them is that one pipeline, so a test
//! drives exactly the code a real invocation takes, with buffers instead of a terminal.

use std::io::Write;

use clifacet::App;
use clifacet::Command;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Dialog, Prompter, Terminal};

use crate::commands::{Cli, Example, ExampleInput, ExampleOutput};
use crate::exit::EXIT;
use crate::interaction::{POLICY, requested_mode};

/// The application: parse [`Cli`], settle the mode against [`POLICY`], ask, validate,
/// run [`Example`], write its output and answer with [`EXIT`]'s codes.
///
/// `main.rs` is `app().run()`. The output is written with the runner's default — its
/// `Display`, one greeting per line — and `--tui` is hosted by the runner, which draws
/// the form generated from `Cli`'s `form` fields on the diagnostic stream.
#[must_use]
pub fn app() -> App<'static, Cli, Example> {
    App::<Cli>::new()
        .command(&Example)
        .policy(POLICY)
        .mode_from(requested_mode)
        .exit(EXIT)
}

/// Runs one parsed invocation, writing to the given streams and returning its exit code.
///
/// Data goes to `out` only, diagnostics to `err` only. Prompts, when the mode permits
/// them, come from the process's terminal.
pub fn run(cli: Cli, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    run_with(cli, Capabilities::detected(), dialog_frontend, out, err)
}

/// The frontend a real process uses: the library's `inquire` dialog.
///
/// Presentation is a choice the application makes here and nowhere else. The dialog
/// asks the same two questions [`terminal_frontend`] does — a value and a yes/no —
/// and differs only in how they look: the session still decides *whether* a question
/// may be asked, and a supplied value is still never re-asked.
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

/// Runs one parsed invocation against explicit capabilities and an explicit frontend.
///
/// Capabilities decide whether prompting is possible at all, and the frontend builder
/// is called only when the resolved plan asks the user something, so a scripted
/// frontend exercises exactly the code a terminal frontend would.
pub fn run_with(
    cli: Cli,
    capabilities: Capabilities,
    build_frontend: impl FnOnce() -> Box<dyn Prompter>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    run_with_command(&Example, cli, capabilities, build_frontend, out, err)
}

/// Runs one parsed invocation of any command that accepts this example's input.
///
/// The example's own [`Example`] is the ordinary case; a different command with the
/// same input and output types is how a caller observes what the run did — see
/// [`clifacet::harness::Recording`]. Because the input type is fixed, "a different
/// command" cannot quietly be a different pipeline: it is the same runner.
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
    let globals = cli.interaction.globals.clone();
    app()
        .command(command)
        .run_parsed(cli, globals, capabilities, build_frontend, out, err)
}

/// Parses process arguments and runs them.
///
/// [`app`]'s `run` with the streams passed in, so a test can hand it its own arguments
/// and buffers while still detecting capabilities and prompting on the process's
/// terminal as a real binary does.
pub fn main(
    argv: impl IntoIterator<Item = std::ffi::OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    app().run_with(argv, Capabilities::detected(), dialog_frontend, out, err)
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
    app()
        .command(command)
        .run_with(argv, capabilities, build_frontend, out, err)
}
