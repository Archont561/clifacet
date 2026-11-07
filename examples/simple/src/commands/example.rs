//! The `example` command: its `clap` surface, its validated input, and the command.
//!
//! Everything about the command lives here and nothing about running it does: no
//! stream, no mode, no exit code. Those are the application's, in the modules next
//! to `commands/`.

use std::fmt;
use std::io::Write;

use clifacet::Command;
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

/// The `clap` surface of the example command, each argument declared once.
///
/// `#[clifacet::command]` derives `clap`'s parser and [`Facet`](clifacet::Facet)
/// together. Every field's `#[clifacet(..)]` carries the whole argument: the grammar
/// keys reach `clap` untouched — aliases, the `--repeat` default and the [`Level`]
/// values stay `clap`'s — and the rest generate the [`TryFrom<Cli>`] bridge to
/// [`ExampleInput`], the questions [`Collect`](clifacet::prompt::Collect) asks, and the
/// form's field list ([`Facet::form_fields`](clifacet::Facet::form_fields)).
#[derive(Debug, Clone, PartialEq, Eq)]
#[clifacet::command(input = ExampleInput, check = check_input)]
#[command(
    name = "example",
    version = clifacet::VERSION,
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
    /// one validation rule. Asked for when no flag supplied it, then filled, trimmed
    /// and refused if blank — whether it came from a flag, an answer or neither.
    #[clifacet(
        long,
        short = 'n',
        alias = "nickname",
        value_name = "NAME",
        ask = "Who should I greet?",
        fill = "world",
        trim,
        non_empty,
        form
    )]
    pub name: Option<String>,

    /// How many times to greet, from 1 to 8.
    ///
    /// A `u8`, so `255` parses: the bound is [`check_input`]'s, in the command's words.
    #[clifacet(long, value_name = "COUNT", default_value_t = 1, form)]
    pub repeat: u8,

    /// How much detail to report.
    #[clifacet(
        long,
        value_name = "LEVEL",
        default_value_t = Level::Basic,
        form
    )]
    pub level: Level,

    /// Fail on purpose, to demonstrate the execution-failure seam.
    ///
    /// Confirmed before anything runs when someone can answer; a non-interactive
    /// `--explode` is a deliberate request, not something to confirm.
    #[clifacet(long, confirm = "Fail on purpose?")]
    pub explode: bool,

    /// An API credential, never echoed back.
    ///
    /// `Secret` is why: the framework prints values through `Debug` and `Display`, and
    /// both of this type's implementations are `[redacted]`. A credential beginning
    /// with `revoked` is refused, to demonstrate that even a diagnostic which names
    /// the credential does not print it. Passing a credential as a flag is visible to
    /// other processes on many systems; see `docs/command-core.md`. Blank is not a
    /// credential: whitespace is trimmed, and nothing left means none was supplied.
    #[clifacet(long, value_name = "TOKEN", secret, trim, non_empty)]
    pub token: Option<Secret<String>>,

    /// How the user wants to be asked, if at all.
    ///
    /// Presentation, not command input: `skip` keeps it out of [`ExampleInput`], and
    /// the application refuses what it cannot honor instead of ignoring it.
    #[command(flatten)]
    #[clifacet(skip)]
    pub interaction: InteractionFlags,
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

/// The one rule the declarations cannot state: `--repeat` within `1..=MAX_REPEAT`.
///
/// The generated [`TryFrom<Cli>`] calls it last, on the input the field rules
/// already built — the name filled, trimmed and known not to be blank, the
/// credential trimmed — so it is the only validation this crate writes by hand, and
/// it reads the value the command will run with rather than the raw flag.
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
