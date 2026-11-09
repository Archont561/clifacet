//! The full-screen form as a test drives it: a screen buffer and a script of keys.
//!
//! A real `--tui` run is hosted by the runner ([`crate::app`]), which reads the
//! terminal's own keys. These entry points take the screen and the keys as parameters
//! instead, so a test can draw the same generated form and submit it. The form produces
//! arguments and `clap` parses them, so a submitted form reaches the same bridge, the
//! same command and the same exit codes as the flags do.

use std::io::Write;

use clap::Parser;

use clifacet::{Command, Facet, Failure};

use crate::commands::{Example, ExampleInput, ExampleOutput, example_command};
use crate::exit::{EXIT_SUCCESS, report};

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
///
/// [`exit_code`]: crate::exit_code
pub fn run_tui<W: Write>(
    screen: W,
    keys: impl IntoIterator<Item = clifacet::tui::Key>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    run_tui_with(&example_command(), screen, keys, out, err)
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
pub fn run_tui_with<W: Write, C>(
    command: &C,
    screen: W,
    keys: impl IntoIterator<Item = clifacet::tui::Key>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32
where
    C: Command<Input = ExampleInput, Output = ExampleOutput>,
{
    let mut state = tui_form();
    let outcome = clifacet::tui::run_on(screen, &mut state, keys, tui_submit_with(command));
    finish_tui(outcome, out, err)
}

/// Writes what the form produced, or reports why it did not finish.
///
/// The one place a form's outcome becomes an exit code, so the two entry points above
/// cannot drift apart.
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
                report(&failure, out, err)
            }
        },
        Err(failure) => report(&failure, out, err),
    }
}

/// The form this command asks, generated from the command's own declarations.
///
/// The fields are the ones [`Example`] declares with `form`, in declaration order
/// ([`Facet::form_fields`]): only the value arguments, because the flags that chose
/// the mode are not questions for a user who already chose one. Each is drawn as what
/// its grammar says — the level, a [`clap::ValueEnum`], is a choice — and a declared
/// `secret` would be masked ([`clifacet::tui::FormState::of`]).
///
/// # Panics
///
/// Panics if the command has none of its form fields, or one of them is a list. The
/// list is generated from the same fields the command is and the derive refuses `form`
/// on a list, so that cannot happen short of a bug in the derive;
/// [`clifacet::tui::FormState::of`] still reports it as a failure for anyone generating
/// a form from someone else's command.
#[must_use]
pub fn tui_form() -> clifacet::tui::FormState {
    clifacet::tui::FormState::of::<Example>()
        .expect("the form fields are generated from the command's own arguments")
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
pub fn tui_submit(state: &clifacet::tui::FormState) -> Result<Vec<String>, String> {
    tui_submit_with(&example_command())(state)
}

/// The submission path, for a command other than the example's own.
///
/// The closure it returns is the same one [`tui_submit`] is: answers become arguments,
/// `clap` parses them, the validation bridge runs, and the command executes.
pub fn tui_submit_with<C>(
    command: &C,
) -> impl Fn(&clifacet::tui::FormState) -> Result<Vec<String>, String> + '_
where
    C: Command<Input = ExampleInput, Output = ExampleOutput>,
{
    move |state: &clifacet::tui::FormState| {
        let argv = std::iter::once(String::from("example")).chain(state.arguments());
        let cli = match Example::try_parse_from(argv) {
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
fn write_lines(lines: &[String], out: &mut dyn Write) -> std::io::Result<()> {
    for line in lines {
        writeln!(out, "{line}")?;
    }
    Ok(())
}
