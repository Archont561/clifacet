//! The prompt adapter's promise: ask only when allowed, and only for what is missing.
//!
//! decision-3 keeps the mode decision and the frontend apart. This file tests the
//! frontend side of that line — the session that decides whether a question may be
//! asked at all, the scripted adapter the framework owns for tests, and the
//! terminal adapter driven here through in-memory streams instead of a TTY.

use std::cell::Cell;
use std::io::Cursor;

use clifacet::policy::{Capabilities, Mode, Plan, Policy, Request};
use clifacet::prompt::{Answer, Scripted, Session, Terminal};
use clifacet::{Command, Failure, FailureKind};
use rstest::rstest;

/// A plan that may ask the user anything.
fn interactive() -> Plan {
    Policy::up_to(Mode::Interactive)
        .resolve(Request::none(), Capabilities::terminal())
        .expect("prompts are permitted here")
}

/// A plan that may ask nothing at all.
fn non_interactive() -> Plan {
    Policy::non_interactive()
        .resolve(Request::none(), Capabilities::headless())
        .expect("the default resolves")
}

/// A plan whose confirmations answer themselves.
fn interactive_assuming_yes() -> Plan {
    Policy::up_to(Mode::Interactive)
        .resolve(Request::none().assuming_yes(true), Capabilities::terminal())
        .expect("prompts are permitted here")
}

#[rstest]
#[case::prompted(interactive(), None, Some("ada"), 1)]
#[case::supplied_is_not_re_asked(interactive(), Some("ada"), Some("ada"), 0)]
#[case::nothing_can_ask(non_interactive(), None, None, 0)]
#[case::supplied_without_a_frontend(non_interactive(), Some("ada"), Some("ada"), 0)]
fn a_missing_value_is_prompted_only_when_permitted(
    #[case] plan: Plan,
    #[case] supplied: Option<&str>,
    #[case] expected: Option<&str>,
    #[case] expected_asks: usize,
) {
    let script = Scripted::new([Answer::Text(String::from("ada"))]);
    let probe = script.clone();
    let mut session = Session::start(plan, move || Box::new(probe));

    let collected = session
        .value(supplied.map(str::to_owned), "Who should I greet?")
        .expect("this answer is usable");

    assert_eq!(collected.as_deref(), expected);
    assert_eq!(
        script.asked().len(),
        expected_asks,
        "asked: {:?}",
        script.asked()
    );
}

#[rstest]
#[case::answers_yes(interactive(), Answer::Yes, true, 1)]
#[case::answers_no(interactive(), Answer::No, false, 1)]
#[case::yes_flag_skips_the_question(interactive_assuming_yes(), Answer::No, true, 0)]
#[case::nothing_can_ask(non_interactive(), Answer::Yes, false, 0)]
fn a_confirmation_respects_the_mode_and_the_yes_flag(
    #[case] plan: Plan,
    #[case] answer: Answer,
    #[case] expected: bool,
    #[case] expected_asks: usize,
) {
    let script = Scripted::new([answer]);
    let probe = script.clone();
    let mut session = Session::start(plan, move || Box::new(probe));

    let confirmed = session
        .confirm("Fail on purpose?")
        .expect("this answer works");

    assert_eq!(confirmed, expected);
    assert_eq!(
        script.asked().len(),
        expected_asks,
        "asked: {:?}",
        script.asked()
    );
}

/// "No" and "I stopped" are different answers: one is a decision the flow may act
/// on, the other ends the flow. Only the second is a cancellation.
#[test]
fn a_dismissed_confirmation_is_a_cancellation_not_a_no() {
    let declined = Scripted::new([Answer::No]);
    let mut session = Session::start(interactive(), {
        let probe = declined.clone();
        move || Box::new(probe)
    });
    assert!(
        !session
            .confirm("Fail on purpose?")
            .expect("no is an answer"),
        "a refusal to proceed is an answer the caller acts on"
    );

    let dismissed = Scripted::new([Answer::Cancel]);
    let mut session = Session::start(interactive(), {
        let probe = dismissed.clone();
        move || Box::new(probe)
    });
    let failure = session
        .confirm("Fail on purpose?")
        .expect_err("dismissing ends the flow");

    assert_eq!(failure.kind(), FailureKind::Cancelled);
    assert_ne!(failure.kind(), FailureKind::Validation);
}

#[test]
fn a_cancelled_question_reports_cancellation() {
    let script = Scripted::new([Answer::Cancel]);
    let probe = script.clone();
    let mut session = Session::start(interactive(), move || Box::new(probe));

    let failure = session
        .value(None, "Who should I greet?")
        .expect_err("a cancel produces no value");

    assert_eq!(failure.kind(), FailureKind::Cancelled);
    assert!(
        !failure.message().is_empty(),
        "a cancel says what was stopped"
    );
}

#[test]
fn a_script_answering_the_wrong_kind_of_question_fails_loudly() {
    let script = Scripted::new([Answer::Yes]);
    let probe = script.clone();
    let mut session = Session::start(interactive(), move || Box::new(probe));

    let failure = session
        .value(None, "Who should I greet?")
        .expect_err("a confirmation is not a name");

    assert_eq!(failure.kind(), FailureKind::Execution);
    assert!(
        failure.message().contains("not text"),
        "the message explains the mismatch: {}",
        failure.message()
    );
}

#[test]
fn a_script_that_runs_out_fails_instead_of_guessing() {
    let script = Scripted::default();
    let probe = script.clone();
    let mut session = Session::start(interactive(), move || Box::new(probe));

    let failure = session
        .value(None, "Who should I greet?")
        .expect_err("an exhausted script cannot answer");

    assert_eq!(failure.kind(), FailureKind::Execution);
    assert!(
        failure.message().contains("script"),
        "the message names the cause: {}",
        failure.message()
    );
}

#[rstest]
#[case::yes("y\n", true)]
#[case::yes_word("yes\n", true)]
#[case::no("n\n", false)]
#[case::no_word("NO\n", false)]
#[case::repeats_until_clear("maybe\nyes\n", true)]
fn a_terminal_confirmation_reads_the_answer(#[case] typed: &str, #[case] expected: bool) {
    let mut prompt = Vec::new();
    let confirmed = {
        let mut terminal = Terminal::new(Cursor::new(typed), &mut prompt);
        clifacet::prompt::Prompter::confirm(&mut terminal, "Fail on purpose?")
    };

    assert_eq!(
        confirmed.expect("the input ends with a clear answer"),
        expected
    );
    let shown = String::from_utf8(prompt).expect("the prompt stream is UTF-8");
    assert!(shown.contains("Fail on purpose?"), "shown: {shown}");
}

#[test]
fn a_terminal_question_keeps_the_answer_and_trims_it() {
    let mut prompt = Vec::new();
    let answer = {
        let mut terminal = Terminal::new(Cursor::new("  ada  \n"), &mut prompt);
        clifacet::prompt::Prompter::ask(&mut terminal, "Who should I greet?")
    };

    assert_eq!(answer.expect("a line was given"), "ada");
}

#[test]
fn end_of_input_is_a_cancellation_not_an_empty_answer() {
    let mut prompt = Vec::new();
    let answer = {
        let mut terminal = Terminal::new(Cursor::new(""), &mut prompt);
        clifacet::prompt::Prompter::ask(&mut terminal, "Who should I greet?")
    };

    assert_eq!(
        answer.expect_err("closing the input cancels").kind(),
        FailureKind::Cancelled
    );
}

/// Counts executions, so "the handler did not run" is observable.
#[derive(Default)]
struct Counting {
    runs: Cell<usize>,
}

impl Command for Counting {
    type Input = ();
    type Output = usize;

    fn execute(&self, (): ()) -> Result<Self::Output, Failure> {
        self.runs.set(self.runs.get() + 1);
        Ok(self.runs.get())
    }
}

#[test]
fn a_collection_failure_never_reaches_the_handler() {
    let command = Counting::default();
    let script = Scripted::default();
    let probe = script.clone();
    let mut session = Session::start(interactive(), move || Box::new(probe));

    let outcome = session.run(&command, |_session| {
        Err(Failure::validation("the answer was no good"))
    });

    assert_eq!(
        outcome.expect_err("collection failed").kind(),
        FailureKind::Validation
    );
    assert_eq!(command.runs.get(), 0, "nothing executed");
}

#[test]
fn a_collected_input_runs_exactly_once() {
    let command = Counting::default();
    let script = Scripted::default();
    let probe = script.clone();
    let mut session = Session::start(interactive(), move || Box::new(probe));

    let runs = session
        .run(&command, |_session| Ok(()))
        .expect("collection succeeded");

    assert_eq!(runs, 1);
    assert_eq!(command.runs.get(), 1);
}

#[test]
fn a_session_reports_the_plan_it_was_given() {
    let script = Scripted::default();
    let probe = script.clone();

    let session = Session::start(interactive(), move || Box::new(probe));

    assert_eq!(session.plan(), interactive());
    assert!(session.plan().asks_the_user());
}

#[test]
fn a_script_reports_how_many_answers_it_has_left() {
    let script = Scripted::new([Answer::Text(String::from("ada")), Answer::Cancel]);
    assert_eq!(script.remaining(), 2);

    let probe = script.clone();
    let mut session = Session::start(interactive(), move || Box::new(probe));
    session
        .value(None, "Who should I greet?")
        .expect("the answer exists");

    assert_eq!(script.remaining(), 1, "an answer is consumed, not copied");
    assert_eq!(script.asked(), vec![String::from("Who should I greet?")]);
}

#[test]
fn a_text_answer_to_a_confirmation_fails_loudly() {
    let script = Scripted::new([Answer::Text(String::from("maybe"))]);
    let probe = script.clone();
    let mut session = Session::start(interactive(), move || Box::new(probe));

    let failure = session
        .confirm("Fail on purpose?")
        .expect_err("text is not a confirmation");

    assert_eq!(failure.kind(), FailureKind::Execution);
    assert!(
        failure.message().contains("yes or no"),
        "the message explains the mismatch: {}",
        failure.message()
    );
}

#[test]
fn a_session_explains_itself_when_debugged() {
    let script = Scripted::default();
    let probe = script.clone();
    let session = Session::start(interactive(), move || Box::new(probe));

    let described = format!("{session:?}");
    assert!(described.contains("Session"), "{described}");
    assert!(described.contains("constructed"), "{described}");
}
