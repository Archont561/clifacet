//! Executable description of the interaction policy's user-visible promise.
//!
//! The steps drive `example::main`, the same entry point a process uses, with
//! buffers instead of a terminal. That is deliberate: a promise about not
//! prompting cannot be tested by a helper that already knows the answer.

use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Scripted};
use clifacet_example as example;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// Anything that starts an ANSI escape sequence, which only a screen draws.
const ESC: char = '\u{1b}';

/// What one run produced, shared between the steps of a scenario.
#[derive(Default)]
struct Run {
    argv: Vec<String>,
    /// The environment the run is told it has, when a scenario states one.
    ///
    /// `None` means the process's own, which is what `example::main` detects; a
    /// scenario that needs a terminal says so here instead of depending on the
    /// machine running the suite.
    capabilities: Option<Capabilities>,
    answers: Vec<Answer>,
    code: i32,
    data: String,
    diagnostic: String,
}

impl Run {
    /// Executes the collected argv, filling in the observable results.
    fn execute(&mut self) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let answers = self.answers.clone();
        let code = match self.capabilities {
            Some(capabilities) => example::main_with(
                self.argv.clone(),
                capabilities,
                move || Box::new(Scripted::new(answers)),
                &mut out,
                &mut err,
            ),
            None => example::main(
                self.argv.iter().map(std::ffi::OsString::from),
                &mut out,
                &mut err,
            ),
        };
        self.code = code;
        self.data = String::from_utf8_lossy(&out).into_owned();
        self.diagnostic = String::from_utf8_lossy(&err).into_owned();
    }
}

#[fixture]
fn run() -> Run {
    Run::default()
}

#[given("an application that runs non-interactively")]
fn the_application_runs_non_interactively(run: &mut Run) {
    run.argv.push(String::from("example"));
}

#[when("the user asks for a TUI")]
fn the_user_asks_for_a_tui(run: &mut Run) {
    run.argv.push(String::from("--tui"));
    run.execute();
}

#[given("a terminal that could host a full-screen form")]
fn a_terminal_that_could_host_a_full_screen_form(run: &mut Run) {
    run.capabilities = Some(Capabilities::terminal_with_tui());
    run.answers = vec![Answer::Text(String::from("ada"))];
}

#[when("the user runs with no flags")]
fn the_user_runs_with_no_flags(run: &mut Run) {
    run.execute();
}

#[then("the answer becomes the data")]
fn the_answer_becomes_the_data(run: &Run) {
    assert_eq!(
        run.data, "hello, ada\n",
        "the prompted answer is the input the command ran on"
    );
}

#[then("no screen is drawn")]
fn no_screen_is_drawn(run: &Run) {
    assert!(
        !run.diagnostic.contains(ESC),
        "a preference for prompts must not draw the form, got {:?}",
        run.diagnostic
    );
}

#[when("the user runs with --yes")]
fn the_user_runs_with_yes(run: &mut Run) {
    run.argv.push(String::from("--yes"));
    run.execute();
}

#[then("nothing is written to the data stream")]
fn nothing_is_written_to_the_data_stream(run: &Run) {
    assert!(
        run.data.is_empty(),
        "a refusal carries no data, got {:?}",
        run.data
    );
}

#[then("the diagnostic names the TUI")]
fn the_diagnostic_names_the_tui(run: &Run) {
    assert!(
        run.diagnostic.contains("TUI"),
        "the diagnostic explains what was refused, got {:?}",
        run.diagnostic
    );
}

#[then("the exit code says nothing ran")]
fn the_exit_code_says_nothing_ran(run: &Run) {
    assert_eq!(run.code, example::EXIT_VALIDATION_FAILED);
}

#[then("the greeting is data, not a question")]
fn the_greeting_is_data(run: &Run) {
    assert!(
        run.data.contains("hello, world"),
        "--yes must not turn the run into a prompt, got {:?}",
        run.data
    );
    assert!(
        run.diagnostic.is_empty(),
        "--yes is a preference, not a diagnostic, got {:?}",
        run.diagnostic
    );
}

#[then("the exit code says the run succeeded")]
fn the_exit_code_says_the_run_succeeded(run: &Run) {
    assert_eq!(run.code, example::EXIT_SUCCESS);
}

#[scenario(
    path = "features/interaction_policy.feature",
    name = "A request for a TUI is refused before anything runs"
)]
#[test]
fn a_refused_mode_runs_nothing(run: Run) {
    let _ = run;
}

#[scenario(
    path = "features/interaction_policy.feature",
    name = "An application that prefers prompts asks instead of drawing a screen"
)]
#[test]
fn a_preference_for_prompts_asks_rather_than_draws(run: Run) {
    let _ = run;
}

#[scenario(
    path = "features/interaction_policy.feature",
    name = "--yes answers confirmations and leaves the mode alone"
)]
#[test]
fn yes_keeps_the_default_mode(run: Run) {
    let _ = run;
}
