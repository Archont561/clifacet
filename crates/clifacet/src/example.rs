//! The reference application: one domain-neutral command, end to end.
//!
//! This module is the worked example M1 asks for, and the shape an application is
//! meant to copy. It holds the three layers apart on purpose:
//!
//! 1. [`Cli`] is the `clap` surface: flags, help and usage errors.
//! 2. [`ExampleInput`] is validated input, built by [`TryFrom<Cli>`] — the
//!    documented bridge the derived [`Facet`] calls. Everything that can be wrong
//!    with a value is rejected there, so [`Input`] is total.
//! 3. [`Example`] is the command: it implements [`Command`], touches no stream and
//!    knows nothing about `clap`.
//!
//! [`run`] is the application layer that joins them: it validates, settles the
//! interaction mode against [`POLICY`], maps failures to the exit codes below and
//! writes machine data to one stream and diagnostics to the other.
//! None of that lives in [`Command`], which is what keeps the seam UI-free.
//!
//! [`Facet`]: crate::Facet
//! [`Input`]: Command::Input

use std::fmt;
use std::io::Write;
use std::str::FromStr;

use clap::Parser;

use crate::command::Command;
use crate::facet::Facet;
use crate::failure::{Failure, FailureKind};
use crate::policy::{self, Capabilities, Mode, Policy, Request};

/// The process exited after doing what was asked.
pub const EXIT_SUCCESS: i32 = 0;

/// The command ran and could not complete.
pub const EXIT_EXECUTION_FAILED: i32 = 1;

/// The input failed validation before the command ran.
pub const EXIT_VALIDATION_FAILED: i32 = 2;

/// `clap` rejected the arguments, or the user asked for help or the version.
///
/// The same value as validation on purpose: both mean "nothing ran, fix the
/// invocation", and `clap` itself documents 2 for usage errors.
pub const EXIT_USAGE: i32 = 2;

/// The user cancelled an interactive flow, so nothing ran.
///
/// `130` is the conventional shell code for interruption, and it is deliberately
/// not `2`: a cancellation is a decision, not a mistake in the invocation.
pub const EXIT_CANCELLED: i32 = 130;

/// The largest `--repeat` this command accepts.
pub const MAX_REPEAT: u8 = 8;

/// How much detail the command reports.
///
/// Parsed by [`parse_level`] rather than by a derived parser, so the error text is
/// the command's own. An unknown level never reaches validation: `clap` rejects it
/// while parsing, which is the whole point of a value parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Level {
    /// One greeting. The default.
    #[default]
    Basic,
    /// The greeting plus the repeat count.
    Verbose,
}

impl fmt::Display for Level {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Basic => "basic",
            Self::Verbose => "verbose",
        })
    }
}

impl FromStr for Level {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "basic" => Ok(Self::Basic),
            "verbose" => Ok(Self::Verbose),
            other => Err(format!(
                "`{other}` is not a level; expected basic or verbose"
            )),
        }
    }
}

/// Parses a [`Level`], so `clap` reports [`Level`]'s own wording.
fn parse_level(value: &str) -> Result<Level, String> {
    value.parse()
}

/// The `clap` surface of the example command.
///
/// The derived [`Facet`] points at [`ExampleInput`] and at [`build_input`], so the
/// only route to typed input is the bridge below; aliases, the `--repeat` default
/// and the [`Level`] parser all stay `clap`'s.
#[derive(Debug, Clone, PartialEq, Eq, Parser, crate::Facet)]
#[facet(input = ExampleInput, bridge = build_input)]
#[command(
    name = "example",
    version = crate::VERSION,
    about = "Clifacet's domain-neutral example command",
    long_about = "Clifacet's domain-neutral example command.\n\n\
                  It exists to show the seams: clap parses, the input is validated, \
                  a UI-free command runs, and the application decides which stream \
                  gets the data and which exit code a failure costs."
)]
pub struct Cli {
    /// Who to greet. Defaults to `world`.
    ///
    /// `--nickname` is an alias and `-n` a short flag: three spellings, one field,
    /// one validation rule.
    #[arg(long, short = 'n', alias = "nickname", value_name = "NAME")]
    pub name: Option<String>,

    /// How many times to greet, from 1 to 8.
    #[arg(long, value_name = "COUNT", default_value_t = 1)]
    pub repeat: u8,

    /// How much detail to report.
    #[arg(long, value_name = "LEVEL", default_value_t = Level::Basic, value_parser = parse_level)]
    pub level: Level,

    /// Fail on purpose, to demonstrate the execution-failure seam.
    #[arg(long)]
    pub explode: bool,

    /// How the user wants to be asked, if at all.
    ///
    /// Presentation, not command input: [`ExampleInput`] does not carry any of it,
    /// and the application refuses what it cannot honor instead of ignoring it.
    #[command(flatten)]
    pub interaction: InteractionFlags,
}

/// The interaction flags, kept together because they are one decision.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::Args)]
pub struct InteractionFlags {
    /// Ask for prompts rather than flags.
    #[arg(long, conflicts_with = "tui")]
    pub interactive: bool,

    /// Ask for a full-screen interface.
    ///
    /// Refused for now: 0.1 ships no TUI, and the policy below says so.
    #[arg(long)]
    pub tui: bool,

    /// Answer confirmations with yes instead of asking.
    ///
    /// It decides nothing else — not the mode, not the input, not the exit code.
    #[arg(long)]
    pub yes: bool,
}

/// How interactive this application is willing to be.
///
/// The ceiling is [`Mode::NonInteractive`] because 0.1 has no prompt or TUI
/// frontend to build: claiming otherwise would let a request resolve to a mode
/// that cannot run. CLI-10 raises the ceiling when the prompt adapter exists, and
/// that is the only line that has to change.
pub const POLICY: Policy = Policy::non_interactive();

/// The mode the user asked for, if any.
///
/// `clap` rejects `--interactive --tui` before this runs, so at most one of them
/// is set and the order here never decides anything.
fn request(flags: InteractionFlags) -> Request {
    let request = match (flags.tui, flags.interactive) {
        (true, _) => Request::mode(Mode::Tui),
        (false, true) => Request::mode(Mode::Interactive),
        (false, false) => Request::none(),
    };
    request.assuming_yes(flags.yes)
}

/// Validated input for [`Example`].
///
/// Every value of this type is runnable: the range check and the default have
/// already happened, so [`Command::execute`] has nothing left to reject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleInput {
    /// Who to greet, never empty.
    pub name: String,
    /// How many greetings to produce, from `1` to [`MAX_REPEAT`].
    pub repeat: u8,
    /// Whether to fail on purpose instead of producing output.
    pub explode: bool,
    /// How much detail to report.
    pub level: Level,
}

impl TryFrom<Cli> for ExampleInput {
    type Error = Failure;

    /// Validates the parsed flags.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::validation`] when `--repeat` is outside `1..=MAX_REPEAT`
    /// or `--name` is blank.
    fn try_from(cli: Cli) -> Result<Self, Self::Error> {
        if cli.repeat == 0 || cli.repeat > MAX_REPEAT {
            return Err(Failure::validation(format!(
                "--repeat must be between 1 and {MAX_REPEAT}, got {}",
                cli.repeat
            )));
        }
        let name = cli.name.unwrap_or_else(|| String::from("world"));
        let name = name.trim();
        if name.is_empty() {
            return Err(Failure::validation("--name must not be blank"));
        }
        Ok(Self {
            name: name.to_owned(),
            repeat: cli.repeat,
            explode: cli.explode,
            level: cli.level,
        })
    }
}

/// The documented bridge from the parsed command to validated input.
///
/// `#[derive(Facet)]` names this function, so there is exactly one path from
/// `clap`'s struct to [`ExampleInput`] — reachable through [`Facet::validate`] and
/// nowhere else. It is a free function because that is what the derive calls; the
/// rules themselves stay in [`TryFrom<Cli>`], which has no `clap` dependency of
/// its own beyond the struct.
///
/// # Errors
///
/// Returns whatever [`TryFrom<Cli>`] rejects: a `--repeat` outside `1..=MAX_REPEAT`
/// or a blank `--name`.
pub fn build_input(cli: Cli) -> Result<ExampleInput, Failure> {
    ExampleInput::try_from(cli)
}

/// What the command produces for the application to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleOutput {
    /// One line of machine data per greeting.
    pub lines: Vec<String>,
}

impl ExampleOutput {
    /// Writes the machine data, one line at a time, to the given stream.
    ///
    /// # Errors
    ///
    /// Returns the writer's error. A caller that cannot write its data stream has
    /// nowhere to report success, so the application maps this to an execution
    /// failure rather than pretending the run succeeded.
    pub fn write_to(&self, out: &mut dyn Write) -> std::io::Result<()> {
        for line in &self.lines {
            writeln!(out, "{line}")?;
        }
        Ok(())
    }
}

impl fmt::Display for ExampleOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.lines.join("\n"))
    }
}

/// The domain-neutral example command.
///
/// It is stateless: configuration lives in [`ExampleInput`], so one value can
/// serve every invocation and no frontend has to carry state for it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Example;

impl Command for Example {
    type Input = ExampleInput;
    type Output = ExampleOutput;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        if input.explode {
            return Err(Failure::execution(
                "the example operation failed on purpose (--explode)",
            ));
        }
        let lines = (1..=input.repeat)
            .map(|index| match input.level {
                Level::Basic => format!("hello, {}", input.name),
                Level::Verbose => {
                    format!("hello, {} ({index} of {})", input.name, input.repeat)
                }
            })
            .collect();
        Ok(ExampleOutput { lines })
    }
}

/// Runs one invocation, writing to the given streams and returning its exit code.
///
/// Data goes to `out` only, diagnostics to `err` only, and nothing is read from a
/// terminal, which is what makes a run reproducible in CI and in tests. A failure
/// is already a diagnostic and an exit code by the time this returns; a writer
/// that fails becomes [`EXIT_EXECUTION_FAILED`].
pub fn run(cli: Cli, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let request = request(cli.interaction);
    let input = match Facet::validate(cli) {
        Ok(input) => input,
        Err(failure) => {
            report(&failure, err);
            return exit_code(&failure);
        }
    };

    // One path from a request to a run: a mode this application forbids, or an
    // environment that cannot host it, returns before `execute` is reachable.
    match policy::invoke(&Example, input, request, POLICY, Capabilities::detected()) {
        Ok(output) => match output.write_to(out) {
            Ok(()) => EXIT_SUCCESS,
            Err(error) => {
                let failure = Failure::execution(format!("cannot write the output: {error}"));
                report(&failure, err);
                exit_code(&failure)
            }
        },
        Err(failure) => {
            report(&failure, err);
            exit_code(&failure)
        }
    }
}

/// The exit code a failure costs this application.
///
/// The numbers are the application's, not the library's (decision-3), and each
/// failure category gets its own: cancelled, execution, and anything else — mode
/// refusals and validation — meaning "nothing ran, fix the invocation".
#[must_use]
pub const fn exit_code(failure: &Failure) -> i32 {
    match failure.kind() {
        FailureKind::Cancelled => EXIT_CANCELLED,
        FailureKind::Execution => EXIT_EXECUTION_FAILED,
        FailureKind::Validation => EXIT_VALIDATION_FAILED,
        // Exhaustive here because this crate defines the enum; an application
        // outside it needs a catch-all arm for the same `#[non_exhaustive]` reason,
        // and should report an unknown category as a failure, never as success.
    }
}

/// Parses arguments, runs them, and returns the exit code a `main` should use.
///
/// This is the whole application entry point, split from `main` so tests can pass
/// their own arguments and buffers instead of a terminal and a process status.
pub fn main(
    argv: impl IntoIterator<Item = std::ffi::OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match Cli::try_parse_from(argv) {
        Ok(cli) => run(cli, out, err),
        Err(error) => {
            // `clap` decides the stream as well as the status: help and the
            // version are the result the caller asked for, on the data stream,
            // while a usage error is a diagnostic.
            if error.use_stderr() {
                let _ = write!(err, "{error}");
                EXIT_USAGE
            } else {
                let _ = write!(out, "{error}");
                EXIT_SUCCESS
            }
        }
    }
}

/// One diagnostic line per failure, with the program name the application owns.
fn report(failure: &Failure, err: &mut dyn Write) {
    let _ = writeln!(err, "example: {failure}");
}
