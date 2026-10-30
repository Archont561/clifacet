//! The reference application: one domain-neutral command, end to end.
//!
//! This module is the worked example M1 asks for, and the shape an application is
//! meant to copy. It holds the three layers apart on purpose:
//!
//! 1. [`Cli`] is the `clap` surface: flags, help and usage errors.
//! 2. [`ExampleInput`] is validated input, built by [`TryFrom<Cli>`]. Everything
//!    that can be wrong with a value is rejected here, so [`Input`] is total.
//! 3. [`Example`] is the command: it implements [`Command`], touches no stream and
//!    knows nothing about `clap`.
//!
//! [`run`] is the application layer that joins them: it maps failures to the exit
//! codes below and writes machine data to one stream and diagnostics to the other.
//! None of that lives in [`Command`], which is what keeps the seam UI-free.
//!
//! [`Input`]: Command::Input

use std::fmt;
use std::io::Write;

use clap::Parser;

use crate::command::Command;
use crate::failure::Failure;

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

/// The largest `--repeat` this command accepts.
pub const MAX_REPEAT: u8 = 8;

/// The `clap` surface of the example command.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
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
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,

    /// How many times to greet, from 1 to 8.
    #[arg(long, value_name = "COUNT", default_value_t = 1)]
    pub repeat: u8,

    /// Fail on purpose, to demonstrate the execution-failure seam.
    #[arg(long)]
    pub explode: bool,
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
        })
    }
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
        let line = format!("hello, {}", input.name);
        Ok(ExampleOutput {
            lines: vec![line; usize::from(input.repeat)],
        })
    }
}

/// Runs one invocation, writing to the given streams and returning its exit code.
///
/// Data goes to `out` only, diagnostics to `err` only, and nothing is read from a
/// terminal, which is what makes a run reproducible in CI and in tests. A failure
/// is already a diagnostic and an exit code by the time this returns; a writer
/// that fails becomes [`EXIT_EXECUTION_FAILED`].
pub fn run(cli: Cli, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let input = match ExampleInput::try_from(cli) {
        Ok(input) => input,
        Err(failure) => {
            report(&failure, err);
            return EXIT_VALIDATION_FAILED;
        }
    };

    match Example.execute(input) {
        Ok(output) => match output.write_to(out) {
            Ok(()) => EXIT_SUCCESS,
            Err(error) => {
                report(
                    &Failure::execution(format!("cannot write the output: {error}")),
                    err,
                );
                EXIT_EXECUTION_FAILED
            }
        },
        Err(failure) => {
            report(&failure, err);
            EXIT_EXECUTION_FAILED
        }
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
