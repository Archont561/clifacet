//! Parity between a prompted run and a flag run, as an executable promise.
//!
//! Both scenarios drive `example::main_with` — the same entry point the binary
//! uses — with a terminal capability and a scripted frontend. Nothing here can
//! touch a real terminal, so a scenario that accidentally prompts fails loudly
//! instead of hanging.

use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Scripted};
use clifacet_example as example;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// One run's observable results.
#[derive(Default)]
struct World {
    code: i32,
    data: String,
    diagnostic: String,
}

impl World {
    /// Runs the example with a terminal available and the given scripted answers.
    fn run(&mut self, argv: &[&str], answers: Vec<Answer>) {
        let script = Scripted::new(answers);
        let mut out = Vec::new();
        let mut err = Vec::new();
        self.code = example::main_with(
            std::iter::once(String::from("example")).chain(argv.iter().map(|a| (*a).to_owned())),
            Capabilities::terminal(),
            move || Box::new(script),
            &mut out,
            &mut err,
        );
        self.data = String::from_utf8_lossy(&out).into_owned();
        self.diagnostic = String::from_utf8_lossy(&err).into_owned();
    }
}

#[fixture]
fn world() -> World {
    World::default()
}

#[given("an application that may prompt")]
fn the_application_may_prompt(_world: &mut World) {}

#[when("the user is asked for a name and answers {answer}")]
fn the_user_answers_the_name_question(world: &mut World, answer: String) {
    world.run(&["--interactive"], vec![Answer::Text(answer)]);
}

#[then("the data is what --name {name} would produce")]
fn the_data_matches_the_flag_run(world: &mut World, name: String) {
    let prompted = world.data.clone();
    assert!(
        !prompted.is_empty(),
        "the prompted run produced no data; diagnostic: {}",
        world.diagnostic
    );

    let mut flags = World::default();
    flags.run(&["--name", &name], Vec::new());

    assert_eq!(
        prompted, flags.data,
        "the same value must produce the same data"
    );
    assert_eq!(world.code, flags.code, "and the same exit code");
}

#[when("the user asks to explode and declines the confirmation")]
fn the_user_declines_the_confirmation(world: &mut World) {
    // The name arrives as a flag, so the only question is the confirmation.
    world.run(
        &["--interactive", "--name", "ada", "--explode"],
        vec![Answer::No],
    );
}

#[then("nothing is written to the data stream")]
fn nothing_is_written(world: &World) {
    assert!(
        world.data.is_empty(),
        "a cancelled run writes no data: {}",
        world.data
    );
}

#[then("the exit code says the run was cancelled")]
fn the_exit_code_says_cancelled(world: &World) {
    assert_eq!(world.code, example::EXIT_CANCELLED);
    assert!(
        world.diagnostic.contains("declined"),
        "the diagnostic explains the cancel: {}",
        world.diagnostic
    );
}

#[scenario(
    path = "tests/features/prompting.feature",
    name = "A prompted answer produces the same data as the flag"
)]
#[test]
fn a_prompted_answer_matches_the_flag(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/prompting.feature",
    name = "A declined confirmation cancels the run"
)]
#[test]
fn a_declined_confirmation_cancels(world: World) {
    let _ = world;
}
