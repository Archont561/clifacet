//! One representative command, three ways in, one result.
//!
//! This is what CLI-13 is for: `clifacet::harness` is public, so an application can
//! prove — not promise — that the mode which collected the input changes nothing
//! about what the command received, produced, failed with, or did.
//!
//! The command is the example's own, reached through its public entry points. Each
//! mode is one small closure around the same `Recording`, so the assertions compare
//! like with like.

use clifacet::harness::{Outcome, Parity, Recording, Subject};
use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Scripted};
use clifacet::tui::Key;
use clifacet_example::{self as example, Example, ExampleInput};

/// How a run is described to a reader: the validated input, which is what the
/// command actually received.
fn describe(input: &ExampleInput) -> String {
    format!("{input:?}")
}

/// The argv a run starts from: the program name, then the flags.
fn argv(flags: &[&str]) -> Vec<String> {
    std::iter::once(String::from("example"))
        .chain(flags.iter().map(|flag| (*flag).to_owned()))
        .collect()
}

/// Runs the command as if the flags had been typed, recording what it received.
fn flags(flags: &[&str]) -> Outcome {
    let (command, effects) = Recording::new(Example, describe as fn(&ExampleInput) -> String);
    let argv = argv(flags);

    Outcome::measured(|out, err| {
        example::main_with_command(
            &command,
            argv,
            Capabilities::headless(),
            || Box::new(Scripted::default()),
            out,
            err,
        )
    })
    .with_effects(effects.recorded())
}

/// Runs the command with a scripted prompt frontend, recording what it received.
fn prompted(flags: &[&str], answers: impl IntoIterator<Item = Answer>) -> Outcome {
    let (command, effects) = Recording::new(Example, describe as fn(&ExampleInput) -> String);
    let argv = argv(flags);
    let script = Scripted::new(answers);

    Outcome::measured(|out, err| {
        example::main_with_command(
            &command,
            argv,
            Capabilities::terminal(),
            || Box::new(script.clone()),
            out,
            err,
        )
    })
    .with_effects(effects.recorded())
}

/// Runs the command through the headless form, recording what it received.
fn form(keys: &[Key]) -> Outcome {
    let (command, effects) = Recording::new(Example, describe as fn(&ExampleInput) -> String);
    let keys = keys.to_vec();

    Outcome::measured(|out, err| example::run_tui_with(&command, Vec::new(), keys, out, err))
        .with_effects(effects.recorded())
}

/// The keystrokes that type `text` into the first field and submit.
fn typing(text: &str) -> Vec<Key> {
    text.chars()
        .map(Key::Char)
        .chain(std::iter::once(Key::Enter))
        .collect()
}

#[test]
fn three_modes_reach_the_command_with_the_same_input() {
    let typed = form(&typing("ada"));
    let flagged = flags(&["--name", "ada"]);
    let prompted = prompted(&[], [Answer::Text(String::from("ada"))]);

    assert!(
        !flagged.effects().is_empty(),
        "the command really ran, so its input was recorded"
    );
    Parity::new("flags", flagged)
        .compare("scripted prompt", prompted)
        .compare("headless tui", typed)
        .assert_parity();
}

#[test]
fn the_recorded_input_is_the_validated_input() {
    let outcome = flags(&["--name", "ada", "--repeat", "2"]);

    assert_eq!(
        outcome.effects(),
        [String::from(
            "ExampleInput { name: \"ada\", repeat: 2, explode: false, level: Basic, token: None }"
        )],
        "what reached the command is the same value every mode builds"
    );
}

#[test]
fn a_successful_run_is_identical_in_every_mode() {
    let typed = form(&typing("ada"));
    let flagged = flags(&["--name", "ada"]);

    assert_eq!(typed.exit(), example::EXIT_SUCCESS);
    assert_eq!(typed.data(), "hello, ada\n");
    assert_eq!(typed.diagnostic(), "");

    assert_eq!(typed.differences(&flagged), Vec::<String>::new());
}

#[test]
fn a_validation_failure_is_reported_in_the_same_words_on_the_command_line() {
    let flagged = flags(&["--repeat", "9"]);
    let prompted = prompted(&["--repeat", "9"], [Answer::Text(String::from("ada"))]);

    assert_eq!(flagged.exit(), example::EXIT_VALIDATION_FAILED);
    assert!(
        flagged.effects().is_empty(),
        "a rejected answer never reaches the command"
    );

    // The same input, a different frontend: the words and the exit code agree, and
    // the recorded effects agree that nothing ran.
    Parity::new("flags", flagged)
        .compare("scripted prompt", prompted)
        .assert_parity();
}

#[test]
fn the_form_refuses_with_the_same_words_and_runs_nothing() {
    let (command, effects) = Recording::new(Example, describe as fn(&ExampleInput) -> String);
    let mut state = example::tui_form();
    let mut terminal = clifacet::tui::ratatui::Terminal::new(
        clifacet::tui::ratatui::backend::TestBackend::new(60, 10),
    )
    .expect("a test backend");

    // Move to `--repeat`, replace its default with an out-of-range value, submit, and
    // let the script run out: the form stays open, which is the point.
    let outcome = clifacet::tui::app::run(
        &mut terminal,
        &mut state,
        [
            Key::Tab,
            Key::Backspace,
            Key::Char('9'),
            Key::Enter,
            Key::Escape,
        ],
        example::tui_submit_with(&command),
    );
    let failure = outcome.expect_err("a refused answer does not finish the form");

    let flagged = flags(&["--repeat", "9"]);
    let expected = flagged.diagnostic().trim();
    let shown = state.message().expect("the refusal is shown on the form");

    assert!(
        expected.ends_with("--repeat must be between 1 and 8, got 9"),
        "the flag run explains itself: {expected}"
    );
    assert!(
        expected.ends_with(shown),
        "the form shows the same refusal: {shown}"
    );
    assert_eq!(failure.kind(), clifacet::FailureKind::Cancelled);
    assert!(
        effects.is_empty(),
        "a refused submission does not execute the command"
    );
}

#[test]
fn leaving_the_form_and_dismissing_the_question_are_the_same_outcome() {
    let escaped = form(&[Key::Escape]);
    let dismissed = prompted(&[], [Answer::Cancel]);

    // The two frontends word a cancellation differently by design — one names the
    // question, the other names the form — so the contract is compared, not the prose.
    let parity = Parity::new("scripted prompt", dismissed.clone())
        .comparing([Subject::Exit, Subject::Data, Subject::Effects])
        .compare("headless tui", escaped.clone());

    assert_eq!(parity.check(), Ok(()));
    assert_eq!(dismissed.exit(), example::EXIT_CANCELLED);
    assert_eq!(escaped.exit(), example::EXIT_CANCELLED);
}

#[test]
fn an_execution_failure_costs_the_same_exit_code_whichever_mode_asked() {
    // `--explode` is a confirmation in prompt mode, so the script has to agree — and
    // the flag run names the same person, or the two runs would differ on the input
    // for a reason that has nothing to do with the mode.
    let flagged = flags(&["--name", "ada", "--explode"]);
    let prompted = prompted(
        &["--explode"],
        [Answer::Text(String::from("ada")), Answer::Yes],
    );

    assert_eq!(flagged.exit(), example::EXIT_EXECUTION_FAILED);
    assert_eq!(
        flagged.effects().len(),
        1,
        "the command ran and failed, which is worth recording"
    );

    Parity::new("flags", flagged)
        .compare("scripted prompt", prompted)
        .assert_parity();
}

#[test]
fn no_mode_writes_a_screen_or_a_question_to_the_data_stream() {
    // The stream contract, checked across frontends rather than per mode: what a person
    // sees never lands in what a caller asked for. A full-screen run is the case that
    // would tempt otherwise, since a terminal is exactly where escape sequences go.
    let runs = [
        ("flags", flags(&["--name", "ada"])),
        (
            "scripted prompt",
            prompted(&[], [Answer::Text(String::from("ada"))]),
        ),
        ("headless form", form(&typing("ada"))),
        ("cancelled form", form(&[Key::Escape])),
    ];

    for (label, outcome) in &runs {
        assert!(
            !outcome.data().contains('\u{1b}'),
            "{label} wrote an escape sequence to the data stream"
        );
        assert!(
            !outcome.data().contains("Who should I greet?"),
            "{label} wrote its question to the data stream"
        );
    }

    // And the contract that makes that possible: the modes agree on the data and the
    // exit code, which is what a caller consuming the stream actually depends on.
    let (_, reference) = &runs[0];
    let parity = runs[1..3].iter().fold(
        Parity::new("flags", reference.clone()).comparing([Subject::Exit, Subject::Data]),
        |parity, (label, outcome)| parity.compare(*label, outcome.clone()),
    );

    assert_eq!(parity.check(), Ok(()));
}
