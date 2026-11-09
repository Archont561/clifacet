//! The `example` command: its `clap` surface, its validated input, and the command.
//!
//! Everything about the command lives here and nothing about running it does: no
//! stream, no mode, no exit code. Those are the application's, in the modules next
//! to `commands/`.
//!
//! It is written as a function, which is the whole of its `clap` surface and the whole of
//! what it does: the parameters are the arguments, and the signature is where each one's
//! declaration is (`#[clifacet(..)]`), so there is no second list to keep in step. What the
//! macro generates beside it — [`Example`], the command type, and [`ExampleInput`], the
//! validated input — is what the rest of the application names.

use std::fmt;
use std::io::Write;

use clap::Parser as _;
use clifacet::Failure;
use clifacet::secret::Secret;

use crate::interaction::InteractionFlags;

/// The largest `--repeat` this command accepts.
pub const MAX_REPEAT: u8 = 8;

/// How much detail the command reports.
///
/// A [`clap::ValueEnum`], so the grammar knows its possible values: `clap` lists them
/// in the help and in its refusal, and the full-screen form offers them as a choice
/// instead of a text box. An unknown level never reaches validation: `clap` rejects it
/// while parsing, which is the whole point of a value parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
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

/// The example's command value, for an entry point that runs this application's own command.
///
/// The parsed command line is the command, so a caller with a line has one already; this is
/// for the entry points that take a *command* and would otherwise need a line to get one. It
/// is only a placeholder: every field is replaced by the parse before anything runs.
///
/// # Panics
///
/// Panics if `["example"]` is not a valid invocation, which would mean a declared argument
/// became required without a default. Every argument here has one, so it cannot.
#[must_use]
pub fn example_command() -> Example {
    Example::try_parse_from(["example"]).expect("an empty line is a valid example invocation")
}

/// The example command, its arguments declared once, on the signature.
///
/// The name is `example` and its version is [`clifacet::VERSION`] through the author's own
/// `#[command(..)]`, forwarded to the generated struct exactly as it would be on a struct.
/// The doc comment is the subcommand's description and long help, as it is for a struct.
#[clifacet::command(check = check_input)]
#[command(
    name = "example",
    version = clifacet::VERSION,
    about = "Clifacet's domain-neutral example command",
    long_about = "Clifacet's domain-neutral example command.\n\n\
                  It exists to show the seams: clap parses, the input is validated, \
                  a UI-free command runs, and the application decides which stream \
                  gets the data and which exit code a failure costs."
)]
pub fn example(
    /// The `--tui` form's field list is generated from this same declaration: `form` is what
    /// puts a value on the full-screen form.
    #[clifacet(
        long,
        short = 'n',
        alias = "nickname",
        value_name = "NAME",
        help = "Who to greet, defaulting to `world`",
        ask = "Who should I greet?",
        fill = "world",
        trim,
        non_empty,
        form
    )]
    name: String,
    /// A `u8`, so `255` parses: the bound is [`check_input`]'s, in the command's words.
    #[clifacet(
        long,
        value_name = "COUNT",
        help = "How many times to greet, from 1 to 8",
        default_value_t = 1,
        form
    )]
    repeat: u8,
    #[clifacet(
        long,
        value_name = "LEVEL",
        help = "How much detail to report",
        default_value_t = Level::Basic,
        form
    )]
    level: Level,
    /// Confirmed before anything runs when someone can answer; a non-interactive
    /// `--explode` is a deliberate request, not something to confirm.
    #[clifacet(
        long,
        help = "Fail on purpose, to show the execution-failure seam",
        confirm = "Fail on purpose?"
    )]
    explode: bool,
    /// A credential, never echoed back.
    ///
    /// `Secret` is why: the framework prints values through `Debug` and `Display`, and
    /// both of this type's implementations are `[redacted]`. A credential beginning
    /// with `revoked` is refused, to demonstrate that even a diagnostic which names
    /// the credential does not print it. Passing a credential as a flag is visible to
    /// other processes on many systems; see `docs/command-core.md`. Blank is not a
    /// credential: whitespace is trimmed, and nothing left means none was supplied.
    #[clifacet(
        long,
        value_name = "TOKEN",
        help = "An API credential, never printed",
        secret,
        trim,
        non_empty
    )]
    token: Option<Secret<String>>,
    /// How the user wants to be asked, if at all.
    ///
    /// Presentation, not command input: `skip` keeps it out of [`ExampleInput`] while
    /// `clap` still parses it, so the flags sit on the command — where the application
    /// reads them to settle the interaction mode and where the runner refuses a
    /// `--dry-run` this command cannot describe — but not on the validated input, because
    /// they decided *how* to ask rather than what was asked. The body does not read them,
    /// which is what `skip` is for.
    #[command(flatten)]
    #[clifacet(skip)]
    interaction: InteractionFlags,
) -> Result<ExampleOutput, Failure> {
    if explode {
        return Err(Failure::execution(
            "the example operation failed on purpose (--explode)",
        ));
    }
    // Interpolating the `Secret` itself, not its value: the diagnostic names the
    // credential and still cannot print it.
    if let Some(token) = &token
        && token.expose().starts_with("revoked")
    {
        return Err(Failure::execution(format!(
            "the credential {token} was rejected"
        )));
    }
    let lines = (1..=repeat)
        .map(|index| match level {
            Level::Basic => format!("hello, {name}"),
            Level::Verbose => format!("hello, {name} ({index} of {repeat})"),
        })
        .collect();
    Ok(ExampleOutput { lines })
}

/// The one rule the declarations cannot state: `--repeat` within `1..=MAX_REPEAT`.
///
/// The generated bridge calls it last, on the input the field rules already built — the
/// name filled, trimmed and known not to be blank, the credential trimmed — so it is the
/// only validation this crate writes by hand, and it reads the value the command will run
/// with rather than the raw flag.
///
/// # Errors
///
/// Returns [`Failure::validation`] when `repeat` is outside `1..=MAX_REPEAT`.
pub fn check_input(input: &ExampleInput) -> Result<(), Failure> {
    if input.repeat == 0 || input.repeat > MAX_REPEAT {
        return Err(Failure::validation(format!(
            "--repeat must be between 1 and {MAX_REPEAT}, got {}",
            input.repeat
        )));
    }
    Ok(())
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
