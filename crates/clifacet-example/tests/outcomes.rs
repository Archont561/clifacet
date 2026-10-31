//! The stream and exit contract: what a run may write where, and what it costs.
//!
//! Two halves, and the same rule for both modes: the data stream carries only the data
//! a caller asked for, and everything a person sees — a question, a screen, a
//! diagnostic — goes to the diagnostic stream. The exit codes are the application's,
//! but which outcome costs which code is a policy rather than a scattering of
//! numbers, so it can be read, asserted and changed in one place.

use std::cell::RefCell;
use std::io::{Cursor, Write};
use std::rc::Rc;

use clifacet::Failure;
use clifacet::contract::ExitPolicy;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Scripted};
use clifacet_example::{self as example, Level};

/// Anything that starts an ANSI escape sequence.
const ESC: char = '\u{1b}';

/// Runs the example with flags, returning exit code, data and diagnostic.
fn run(flags: &[&str], capabilities: Capabilities) -> (i32, String, String) {
    run_with(flags, capabilities, || Box::new(Scripted::default()))
}

/// Runs the example with flags and an explicit scripted frontend.
fn run_with(
    flags: &[&str],
    capabilities: Capabilities,
    frontend: impl Fn() -> Box<dyn clifacet::prompt::Prompter>,
) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::main_with(
        std::iter::once(String::from("example")).chain(flags.iter().map(|flag| (*flag).to_owned())),
        capabilities,
        frontend,
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn the_standard_policy_maps_every_outcome() {
    let policy = ExitPolicy::standard();

    assert_eq!(policy.success(), 0);
    assert_eq!(
        policy.for_failure(&Failure::validation("bad value")),
        policy.validation()
    );
    assert_eq!(
        policy.for_failure(&Failure::cancelled("changed their mind")),
        policy.cancelled()
    );
    assert_eq!(
        policy.for_failure(&Failure::execution("it broke")),
        policy.execution()
    );
}

#[test]
fn the_four_outcomes_have_distinct_codes() {
    // Success, validation, cancellation and execution are four different things, so a
    // caller has to be able to tell them apart without reading the diagnostic.
    let policy = ExitPolicy::standard();
    let codes = [
        policy.success(),
        policy.validation(),
        policy.cancelled(),
        policy.execution(),
    ];

    for (index, left) in codes.iter().enumerate() {
        for right in &codes[index + 1..] {
            assert_ne!(left, right, "two outcomes share the code {left}");
        }
    }
}

#[test]
fn the_example_documents_usage_sharing_a_code_with_validation() {
    // A usage error and a validation failure both mean "nothing ran, fix the
    // invocation", and `clap` documents 2 for the first. The overlap is deliberate, so
    // it is asserted rather than left to look like an accident.
    let policy = ExitPolicy::standard();

    assert_eq!(policy.usage(), policy.validation());
    assert_eq!(example::EXIT_USAGE, policy.usage());
    assert_eq!(example::EXIT_VALIDATION_FAILED, policy.validation());
    assert_eq!(example::EXIT_SUCCESS, policy.success());
    assert_eq!(example::EXIT_CANCELLED, policy.cancelled());
    assert_eq!(example::EXIT_EXECUTION_FAILED, policy.execution());
}

#[test]
fn the_example_maps_failures_through_the_policy() {
    let failure = Failure::validation("--repeat must be between 1 and 8, got 9");

    assert_eq!(
        example::exit_code(&failure),
        ExitPolicy::standard().for_failure(&failure),
        "the application answers with the documented policy, not with its own numbers"
    );
}

#[test]
fn a_successful_run_writes_data_and_nothing_else() {
    let (code, data, diagnostic) = run(
        &["--name", "ada", "--level", "verbose"],
        Capabilities::headless(),
    );

    assert_eq!(code, example::EXIT_SUCCESS);
    assert_eq!(data, "hello, ada (1 of 1)\n");
    assert_eq!(diagnostic, "");
    assert!(
        !data.contains(ESC),
        "the data stream carries no escape sequences"
    );
}

/// A writer that keeps what it was given, so a frontend's output can be asserted.
#[derive(Clone, Default)]
struct Shared(Rc<RefCell<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_question_never_reaches_the_data_stream() {
    // A real `Terminal`, not a script: the question is what this asserts about, and a
    // scripted frontend answers silently.
    let transcript = Shared::default();
    let (code, data, diagnostic) = run_with(&[], Capabilities::terminal(), || {
        Box::new(clifacet::prompt::Terminal::new(
            Cursor::new("ada\n"),
            transcript.clone(),
        ))
    });

    let asked = String::from_utf8_lossy(&transcript.0.borrow()).into_owned();

    assert_eq!(code, example::EXIT_SUCCESS);
    assert_eq!(
        data, "hello, ada\n",
        "the data stream holds only the greeting"
    );
    assert!(
        asked.contains("Who should I greet?"),
        "the question goes to the prompt's own stream, never the data: {asked:?}"
    );
    assert_eq!(diagnostic, "", "a successful run reports nothing else");
}

#[test]
fn a_policy_that_cannot_ask_asks_nothing_and_says_nothing() {
    // Headless capabilities: no question is written anywhere, so a piped run cannot
    // block waiting for an answer that will never come.
    let (code, data, diagnostic) = run(&["--explode"], Capabilities::headless());

    assert_eq!(code, example::EXIT_EXECUTION_FAILED);
    assert_eq!(data, "", "a failed run reports nothing as data");
    assert_eq!(
        diagnostic.lines().count(),
        1,
        "one diagnostic line, and no question: {diagnostic:?}"
    );
    assert!(!diagnostic.contains("Fail on purpose?"));
}

#[test]
fn a_usage_error_is_a_diagnostic_and_costs_two() {
    let (code, data, diagnostic) = run(&["--nope"], Capabilities::headless());

    assert_eq!(code, example::EXIT_USAGE);
    assert_eq!(data, "", "a usage error is not data");
    assert!(diagnostic.contains("--nope"));
}

#[test]
fn help_is_the_result_the_caller_asked_for() {
    let (code, data, diagnostic) = run(&["--help"], Capabilities::headless());

    assert_eq!(code, example::EXIT_SUCCESS, "asking for help is a success");
    assert!(
        data.contains("Usage:"),
        "help is the answer, so it is the data"
    );
    assert_eq!(diagnostic, "");
}

#[test]
fn validation_costs_two_and_writes_only_a_diagnostic() {
    let (code, data, diagnostic) = run(&["--repeat", "9"], Capabilities::headless());

    assert_eq!(code, example::EXIT_VALIDATION_FAILED);
    assert_eq!(data, "");
    assert_eq!(
        diagnostic, "example: --repeat must be between 1 and 8, got 9\n",
        "one diagnostic line, with the program name the application owns"
    );
}

#[test]
fn a_cancellation_costs_a_hundred_and_thirty_and_writes_no_data() {
    let (code, data, _) = run_with(&[], Capabilities::terminal(), || {
        Box::new(Scripted::new([Answer::Cancel]))
    });

    assert_eq!(code, example::EXIT_CANCELLED);
    assert_eq!(data, "");
}

#[test]
fn the_full_screen_mode_draws_on_the_diagnostic_stream() {
    use std::io::IsTerminal;

    if std::io::stdin().is_terminal() {
        // A test with a real terminal on stdin would wait for a keypress.
        return;
    }

    let (code, data, diagnostic) = run(&["--tui"], Capabilities::terminal_with_tui());

    assert_eq!(code, example::EXIT_CANCELLED, "the silent terminal cancels");
    assert_eq!(
        data, "",
        "a screen is not data: nothing a user sees reaches the data stream"
    );
    assert!(
        diagnostic.contains("\u{1b}[?1049h"),
        "the alternate screen is entered on the diagnostic stream"
    );
    assert!(diagnostic.contains("\u{1b}[?1049l"), "and left again");
}

#[test]
fn the_full_screen_mode_writes_only_data_when_it_submits() {
    use clifacet::tui::Key;

    let mut out = Vec::new();
    let mut err = Vec::new();
    let keys: Vec<Key> = "ada"
        .chars()
        .map(Key::Char)
        .chain(std::iter::once(Key::Enter))
        .collect();

    let code = example::run_tui(Vec::new(), keys, &mut out, &mut err);

    assert_eq!(code, example::EXIT_SUCCESS);
    assert_eq!(String::from_utf8_lossy(&out), "hello, ada\n");
    assert!(!String::from_utf8_lossy(&out).contains(ESC));
}

#[test]
fn the_examples_verbose_data_is_one_line_per_greeting() {
    let (_, data, _) = run(
        &["--name", "ada", "--repeat", "3"],
        Capabilities::headless(),
    );

    assert_eq!(data.lines().count(), 3, "three greetings, three lines");
    assert!(
        data.ends_with('\n'),
        "each greeting is a complete line, so a reader can stream it"
    );
    assert!(data.lines().all(|line| line == "hello, ada"));
}

#[test]
fn the_level_is_data_and_not_presentation() {
    // `--level` changes what the command produces, so it changes the data. Nothing
    // else about the run changes with it.
    let basic = run(&["--level", "basic"], Capabilities::headless());
    let verbose = run(&["--level", "verbose"], Capabilities::headless());

    assert_eq!(basic.0, verbose.0);
    assert_eq!(basic.2, verbose.2, "the diagnostic is unchanged");
    assert_ne!(basic.1, verbose.1);
    assert_eq!(verbose.1, "hello, world (1 of 1)\n");
    let _ = Level::Basic;
}
