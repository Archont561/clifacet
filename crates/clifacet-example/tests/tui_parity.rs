//! A submitted TUI form is the same command as the flags, end to end.
//!
//! The screen is a writer the test owns, and the keys are a script, so the whole
//! TUI path — form, submission, validation, handler, exit code — runs without a
//! terminal. What it must prove is that the answer a user gets does not depend on
//! which frontend collected it.

use clifacet::policy::Capabilities;
use clifacet::prompt::Scripted;
use clifacet::tui::{Key, app};
use clifacet_example as example;
use rstest::rstest;

/// Runs the example as a flag invocation, returning exit code, data and diagnostic.
fn flags(argv: &[&str]) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::main_with(
        std::iter::once(String::from("example")).chain(argv.iter().map(|a| (*a).to_owned())),
        Capabilities::headless(),
        || Box::new(Scripted::default()),
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

/// Runs the TUI with the given keystrokes, returning exit code, data and diagnostic.
fn tui(keys: &[Key]) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::run_tui(Vec::new(), keys.iter().copied(), &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

/// The keystrokes that type `text` and submit.
fn typing(text: &str) -> Vec<Key> {
    text.chars()
        .map(Key::Char)
        .chain(std::iter::once(Key::Enter))
        .collect()
}

#[test]
fn the_example_form_holds_its_value_fields_and_not_the_interaction_flags() {
    let state = example::tui_form();
    let names: Vec<&str> = state
        .fields()
        .iter()
        .map(clifacet::tui::FieldState::name)
        .collect();

    assert_eq!(
        names,
        ["name", "repeat", "level"],
        "the flags that chose the mode are not questions for the form"
    );
}

#[test]
fn a_typed_answer_produces_the_same_data_as_the_flag() {
    let typed = tui(&typing("ada"));
    let flagged = flags(&["--name", "ada"]);

    assert_eq!(typed.1, flagged.1, "same data");
    assert_eq!(typed.0, flagged.0, "same exit code");
    assert_eq!(typed.1, "hello, ada\n");
}

#[test]
fn an_edited_default_reaches_the_command() {
    // clear the default repeat (1) and type 3
    let keys = vec![Key::Tab, Key::Backspace, Key::Char('3'), Key::Enter];
    let edited = tui(&keys);
    let flagged = flags(&["--repeat", "3"]);

    assert_eq!(edited.1, flagged.1);
    assert_eq!(edited.1, "hello, world\nhello, world\nhello, world\n");
}

#[rstest]
#[case::out_of_range(vec![Key::Tab, Key::Backspace, Key::Char('9'), Key::Enter], "9")]
#[case::not_a_number(vec![Key::Tab, Key::Backspace, Key::Char('x'), Key::Enter], "x")]
fn a_rejected_answer_keeps_the_form_open_and_shows_the_flag_message(
    #[case] keys: Vec<Key>,
    #[case] refused: &str,
) {
    let mut state = example::tui_form();
    let mut terminal = clifacet::tui::ratatui::Terminal::new(
        clifacet::tui::ratatui::backend::TestBackend::new(60, 10),
    )
    .expect("a test backend");

    // One submission, which must be refused, then the script ends.
    let outcome = app::run(&mut terminal, &mut state, keys, example::tui_submit);

    let failure = outcome.expect_err("a refused submission does not finish the form");
    assert_eq!(
        failure.kind(),
        clifacet::FailureKind::Cancelled,
        "the script ran out after the refusal"
    );
    assert!(
        state.message().is_some(),
        "the refusal is shown on the form rather than printed"
    );

    // The same answer, through the flags, must produce the same words.
    let (_, _, diagnostic) = flags(&["--repeat", refused]);
    let shown = state.message().unwrap_or_default().to_owned();
    assert!(
        diagnostic.contains(&shown),
        "flags said {diagnostic:?}, form said {shown:?}"
    );
}

#[test]
fn escape_cancels_and_runs_nothing() {
    let (code, data, diagnostic) = tui(&[Key::Escape]);

    assert_eq!(code, example::EXIT_CANCELLED);
    assert!(data.is_empty(), "a cancelled run writes no data: {data}");
    assert!(
        diagnostic.contains("cancel"),
        "the diagnostic says what happened: {diagnostic}"
    );
}

#[test]
fn a_script_that_ends_without_submitting_cancels_instead_of_spinning() {
    let (code, data, _) = tui(&[Key::Char('a')]);

    assert_eq!(code, example::EXIT_CANCELLED);
    assert_eq!(data, "", "a cancelled run writes no data");
}

#[test]
fn the_tui_leaves_the_screen_it_entered() {
    let mut screen = Vec::new();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let _ = example::run_tui(&mut screen, std::iter::empty(), &mut out, &mut err);

    let written = String::from_utf8_lossy(&screen).into_owned();
    assert!(
        written.contains("\u{1b}[?1049h") && written.contains("\u{1b}[?1049l"),
        "the alternate screen is entered and left, even on cancellation"
    );
}

/// A writer that refuses everything, standing in for a closed output pipe.
struct Refusing;

impl std::io::Write for Refusing {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_closed_output_stream_is_a_failure_and_not_a_success() {
    let mut err = Vec::new();
    let code = example::run_tui(Vec::new(), typing("ada"), &mut Refusing, &mut err);
    let diagnostic = String::from_utf8_lossy(&err).into_owned();

    assert_eq!(
        code,
        example::EXIT_EXECUTION_FAILED,
        "data that could not be written is not a completed run"
    );
    assert!(
        diagnostic.contains("cannot write the output"),
        "the diagnostic says what failed: {diagnostic}"
    );
}

#[test]
fn asking_for_the_tui_reaches_the_form_and_a_silent_terminal_cancels() {
    use std::io::IsTerminal;

    // `--tui` hands the real terminal to the form. A test process whose stdin is a
    // terminal would sit there waiting for a key, so this only runs where there is
    // none: the form then reads no events and reports a cancellation.
    if std::io::stdin().is_terminal() {
        return;
    }

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::main_with(
        [String::from("example"), String::from("--tui")],
        Capabilities::terminal_with_tui(),
        || Box::new(Scripted::default()),
        &mut out,
        &mut err,
    );

    assert_eq!(code, example::EXIT_CANCELLED);
    assert_eq!(String::from_utf8_lossy(&out), "");
    assert!(
        String::from_utf8_lossy(&err).contains("cancel"),
        "the diagnostic explains the exit: {}",
        String::from_utf8_lossy(&err)
    );
}
