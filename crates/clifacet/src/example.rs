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
use crate::contract::ExitPolicy;
use crate::facet::Facet;
use crate::failure::Failure;
use crate::policy::{Capabilities, Mode, Policy, Request};
use crate::prompt::{Prompter, Session, Terminal};
use crate::secret::Secret;

/// The exit codes this application gives its outcomes.
///
/// The numbers are the application's (decision-3); the categories are the library's.
/// [`ExitPolicy::standard`] is that mapping in one readable place, and the constants
/// below are names for it so a test can point at the outcome it means.
pub const EXIT: ExitPolicy = ExitPolicy::standard();

/// The process exited after doing what was asked.
pub const EXIT_SUCCESS: i32 = EXIT.success();

/// The command ran and could not complete.
pub const EXIT_EXECUTION_FAILED: i32 = EXIT.execution();

/// The input failed validation before the command ran.
pub const EXIT_VALIDATION_FAILED: i32 = EXIT.validation();

/// `clap` rejected the arguments, or the user asked for help or the version.
///
/// The same value as validation on purpose: both mean "nothing ran, fix the
/// invocation", and `clap` itself documents 2 for usage errors.
pub const EXIT_USAGE: i32 = EXIT.usage();

/// The user cancelled an interactive flow, so nothing ran.
///
/// `130` is the conventional shell code for interruption, and it is deliberately
/// not `2`: a cancellation is a decision, not a mistake in the invocation.
pub const EXIT_CANCELLED: i32 = EXIT.cancelled();

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

    /// An API credential, never echoed back.
    ///
    /// `Secret` is why: the framework prints values through `Debug` and `Display`, and
    /// both of this type's implementations are `[redacted]`. A credential beginning
    /// with `revoked` is refused, to demonstrate that even a diagnostic which names
    /// the credential does not print it. Passing a credential as a flag is visible to
    /// other processes on many systems; see `docs/command-core.md`.
    #[arg(long, value_name = "TOKEN")]
    pub token: Option<Secret<String>>,

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
    /// Honoured when the crate is built with the `tui` feature; refused with a
    /// message otherwise, because a mode nothing can draw is not a mode.
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
/// Prompts are permitted — [`run`] supplies a terminal frontend for them. A TUI is
/// permitted only when the `tui` feature is compiled in, because [`run_with`] then
/// has a screen to draw on; without it `--tui` is refused with a message rather than
/// resolving to a mode nothing can host.
pub const POLICY: Policy = Policy::up_to(if cfg!(feature = "tui") {
    Mode::Tui
} else {
    Mode::Interactive
});

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
    /// An API credential, if one was supplied. Never printed.
    pub token: Option<Secret<String>>,
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
        // A blank credential is not a credential: whitespace would otherwise reach the
        // command as a value that looks supplied.
        let token = cli
            .token
            .map(Secret::into_inner)
            .map(|token| token.trim().to_owned())
            .filter(|token| !token.is_empty())
            .map(Secret::new);
        Ok(Self {
            name: name.to_owned(),
            repeat: cli.repeat,
            explode: cli.explode,
            level: cli.level,
            token,
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
        // Interpolating the `Secret` itself, not its value: the diagnostic names the
        // credential and still cannot print it.
        if let Some(token) = &input.token
            && token.expose().starts_with("revoked")
        {
            return Err(Failure::execution(format!(
                "the credential {token} was rejected"
            )));
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
/// Data goes to `out` only, diagnostics to `err` only. Prompts, when the mode
/// permits them, come from the process's terminal: this is the entry point that
/// owns real streams, and it is deliberately thin so that everything below it can
/// be driven by buffers.
pub fn run(cli: Cli, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    run_with(cli, Capabilities::detected(), terminal_frontend, out, err)
}

/// The frontend a real process uses.
///
/// Constructing it locks stdin and holds stderr, but touches neither until a
/// question is actually asked, so this is safe to build and discard.
pub(crate) fn terminal_frontend() -> Box<dyn Prompter> {
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
/// [`crate::harness::Recording`]. Because the input type is fixed, "a different
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
            report(&failure, err);
            return exit_code(&failure);
        }
    };

    // Without the feature the policy never resolves to this mode, so this branch is
    // dead code rather than a fallback — it exists only to keep the two in step.
    #[cfg(feature = "tui")]
    if plan.mode() == Mode::Tui {
        // The screen is drawn on the diagnostic stream the caller passed, like every
        // other thing a person sees. Drawing it on the data stream would put escape
        // sequences in what a caller asked for, which is the one thing the stream
        // contract forbids — and reaching for the process's own stderr would bypass
        // the streams this function was given.
        return run_tui_on(command, err, crate::tui::live_keys(), out);
    }

    let mut session = Session::start(plan, build_frontend);
    match session.run(command, move |session| collect(session, cli)) {
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

/// Fills the draft, then validates it through the same bridge the flags use.
///
/// A value the user supplied is never re-asked, and a missing one is asked for
/// only when the plan permits — which is what makes a prompted run and a flag run
/// the same command rather than two behaviours.
fn collect(session: &mut Session, mut cli: Cli) -> Result<ExampleInput, Failure> {
    let name = session.value(cli.name.take(), "Who should I greet?")?;
    cli.name = name;

    // Only ask when something can answer: a non-interactive `--explode` is a
    // deliberate request, not something to confirm.
    if cli.explode && session.plan().asks_the_user() && !session.confirm("Fail on purpose?")? {
        return Err(Failure::cancelled(
            "the user declined to fail on purpose (--explode)",
        ));
    }

    Facet::validate(cli)
}

/// The exit code a failure costs this application.
///
/// The numbers are the application's, not the library's (decision-3), and each
/// failure category gets its own: cancelled, execution, and anything else — mode
/// refusals and validation — meaning "nothing ran, fix the invocation". The mapping
/// itself lives in [`EXIT`], so it can be read in one place and asserted as a policy.
#[must_use]
pub fn exit_code(failure: &Failure) -> i32 {
    EXIT.for_failure(failure)
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
        Err(error) => report_usage(&error, out, err),
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
        Err(error) => report_usage(&error, out, err),
    }
}

/// Reports a `clap` outcome on the stream and with the status `clap` implies.
fn report_usage(error: &clap::Error, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    // `clap` decides the stream as well as the status: help and the version are
    // the result the caller asked for, on the data stream, while a usage error is
    // a diagnostic.
    if error.use_stderr() {
        let _ = write!(err, "{error}");
        EXIT_USAGE
    } else {
        let _ = write!(out, "{error}");
        EXIT_SUCCESS
    }
}

/// Runs the full-screen form, then reports its outcome like any other run.
///
/// The screen and the keystrokes are parameters so a test drives this exact code
/// with a buffer and a script; a real run passes stdout and the terminal's events.
///
/// # Errors and exit codes
///
/// This is the application layer, so it answers with an exit code: the data on
/// `out`, the diagnostic on `err`, and the same mapping [`exit_code`] applies to
/// every other failure. A cancelled form is a cancellation, not a mistake.
#[cfg(feature = "tui")]
pub fn run_tui<W: Write>(
    screen: W,
    keys: impl IntoIterator<Item = crate::tui::Key>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    run_tui_with(&Example, screen, keys, out, err)
}

/// Runs the form and hands its answers to `command`.
///
/// The screen, the keys and the command are all parameters: a test drives the real
/// form — the same state machine, the same submission path — against a buffer, a
/// script, and a command that records what it received.
///
/// # Errors and exit codes
///
/// As [`run_tui`].
#[cfg(feature = "tui")]
pub fn run_tui_with<W: Write, C>(
    command: &C,
    screen: W,
    keys: impl IntoIterator<Item = crate::tui::Key>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32
where
    C: Command<Input = ExampleInput, Output = ExampleOutput>,
{
    let mut state = tui_form();
    let outcome = crate::tui::run_on(screen, &mut state, keys, tui_submit_with(command));
    finish_tui(outcome, out, err)
}

/// Runs the form with the screen drawn on the diagnostic stream itself.
///
/// A run that asks a person something has one place to show it: the screen and the
/// diagnostic are the same stream, which is what lets a real invocation draw on
/// stderr while a test asserts the escape sequences in a buffer.
#[cfg(feature = "tui")]
fn run_tui_on<C>(
    command: &C,
    stream: &mut dyn Write,
    keys: impl IntoIterator<Item = crate::tui::Key>,
    out: &mut dyn Write,
) -> i32
where
    C: Command<Input = ExampleInput, Output = ExampleOutput>,
{
    let mut state = tui_form();
    let outcome = crate::tui::run_on(&mut *stream, &mut state, keys, tui_submit_with(command));
    finish_tui(outcome, out, stream)
}

/// Writes what the form produced, or reports why it did not finish.
///
/// The one place a form's outcome becomes an exit code, so the two entry points above
/// cannot drift apart.
#[cfg(feature = "tui")]
fn finish_tui(
    outcome: Result<Vec<String>, Failure>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match outcome {
        Ok(lines) => match write_lines(&lines, out) {
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

/// The form this command asks, generated from the command itself.
///
/// Only the value arguments appear: the flags that chose the mode are not questions
/// for a user who already chose one.
///
/// # Panics
///
/// Panics if the command no longer has one of these arguments. The names are a
/// constant beside the command they name, so a mismatch is a bug in this crate, not
/// something a user can cause; [`crate::tui::FormState::generated`] still reports it
/// as a failure for anyone generating a form from someone else's command.
#[cfg(feature = "tui")]
#[must_use]
pub fn tui_form() -> crate::tui::FormState {
    crate::tui::FormState::generated(&<Cli as Facet>::command(), &["name", "repeat", "level"])
        .expect("the example form names arguments its own command has")
}

/// Turns the answers into a run, through the same validation and handler as the flags.
///
/// # Errors
///
/// Returns the message to show on the form when the answers are not a valid
/// invocation: the parser's own words for a malformed value, or the failure the
/// handler reported. Both are the same words the flags would produce.
///
/// The form produces arguments, `clap` parses them — the grammar stays the authority,
/// aliases and value parsers included — and the draft goes through the same bridge
/// every other frontend uses. A refusal is returned as a message for the form to
/// show, because a form that closes on a typo makes the user start again.
#[cfg(feature = "tui")]
pub fn tui_submit(state: &crate::tui::FormState) -> Result<Vec<String>, String> {
    tui_submit_with(&Example)(state)
}

/// The submission path, for a command other than the example's own.
///
/// The closure it returns is the same one [`tui_submit`] is: answers become arguments,
/// `clap` parses them, the validation bridge runs, and the command executes.
#[cfg(feature = "tui")]
pub fn tui_submit_with<C>(
    command: &C,
) -> impl Fn(&crate::tui::FormState) -> Result<Vec<String>, String> + '_
where
    C: Command<Input = ExampleInput, Output = ExampleOutput>,
{
    move |state: &crate::tui::FormState| {
        let argv = std::iter::once(String::from("example")).chain(state.arguments());
        let cli = match Cli::try_parse_from(argv) {
            Ok(cli) => cli,
            Err(error) => return Err(error.to_string()),
        };

        match Facet::validate(cli).and_then(|input| command.execute(input)) {
            Ok(output) => Ok(output.lines),
            Err(failure) => Err(failure.message().to_owned()),
        }
    }
}

/// Writes each line of data, keeping the data stream machine-readable.
#[cfg(feature = "tui")]
fn write_lines(lines: &[String], out: &mut dyn Write) -> std::io::Result<()> {
    for line in lines {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

/// One diagnostic line per failure, with the program name the application owns.
fn report(failure: &Failure, err: &mut dyn Write) {
    let _ = writeln!(err, "example: {failure}");
}
