//! The parity harness's promise, stated as scenarios.
//!
//! Each scenario builds the same three frontends the integration tests build and
//! compares them through `clifacet::harness` — the public API, not a test helper.

#![cfg(feature = "tui")]

use clifacet::example::{self, Example, ExampleInput};
use clifacet::harness::{Outcome, Parity, Recording, Subject};
use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Scripted};
use clifacet::tui::Key;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// What the scenarios collect.
#[derive(Default)]
struct World {
    flagged: Option<Outcome>,
    prompted: Option<Outcome>,
    typed: Option<Outcome>,
}

#[fixture]
fn world() -> World {
    World::default()
}

/// The argv every run starts from.
fn argv(flags: &[&str]) -> Vec<String> {
    std::iter::once(String::from("example"))
        .chain(flags.iter().map(|flag| (*flag).to_owned()))
        .collect()
}

fn describe(input: &ExampleInput) -> String {
    format!("{input:?}")
}

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

fn form(keys: Vec<Key>) -> Outcome {
    let (command, effects) = Recording::new(Example, describe as fn(&ExampleInput) -> String);
    Outcome::measured(|out, err| example::run_tui_with(&command, Vec::new(), keys, out, err))
        .with_effects(effects.recorded())
}

#[given("the example command, recorded through the parity harness")]
fn the_command_is_recorded(world: &mut World) {
    // Nothing to prepare: each step below builds its own recording, because an
    // effects handle belongs to one run and comparing runs must not share one.
    assert!(world.flagged.is_none() && world.prompted.is_none() && world.typed.is_none());
}

#[when("ada is supplied through the flags, the scripted prompt and the form")]
fn ada_through_every_mode(world: &mut World) {
    world.flagged = Some(flags(&["--name", "ada"]));
    world.prompted = Some(prompted(&[], [Answer::Text(String::from("ada"))]));
    world.typed = Some(form(
        "ada".chars().map(Key::Char).chain([Key::Enter]).collect(),
    ));
}

#[then("all three modes agree on the data, the exit code and the recorded input")]
fn the_modes_agree(world: &World) {
    let flagged = world.flagged.as_ref().expect("the flag run happened");
    let prompted = world.prompted.as_ref().expect("the prompt run happened");
    let typed = world.typed.as_ref().expect("the form run happened");

    let parity = Parity::new("flags", flagged.clone())
        .comparing([Subject::Exit, Subject::Data, Subject::Effects])
        .compare("scripted prompt", prompted.clone())
        .compare("headless tui", typed.clone());

    assert_eq!(parity.check(), Ok(()));
    assert_eq!(
        typed.effects(),
        flagged.effects(),
        "the same validated input reached the command"
    );
}

#[when("an out-of-range repeat is supplied through the flags and the scripted prompt")]
fn out_of_range(world: &mut World) {
    world.flagged = Some(flags(&["--repeat", "9"]));
    world.prompted = Some(prompted(
        &["--repeat", "9"],
        [Answer::Text(String::from("ada"))],
    ));
}

#[then("neither run reaches the command")]
fn nothing_ran(world: &World) {
    assert!(
        world
            .flagged
            .as_ref()
            .expect("flag run")
            .effects()
            .is_empty(),
        "a rejected flag invocation never executes the command"
    );
    assert!(
        world
            .prompted
            .as_ref()
            .expect("prompt run")
            .effects()
            .is_empty(),
        "a rejected prompt was collected before execution and nothing ran"
    );
}

#[then("both report the same failure and exit code")]
fn both_fail_alike(world: &World) {
    let parity = Parity::new("flags", world.flagged.clone().expect("flag run"))
        .comparing([Subject::Exit, Subject::Diagnostic, Subject::Effects])
        .compare(
            "scripted prompt",
            world.prompted.clone().expect("prompt run"),
        );

    assert_eq!(parity.check(), Ok(()));
}

#[scenario(
    path = "tests/features/parity.feature",
    name = "Every mode reaches the command with the same answer"
)]
#[test]
fn every_mode_reaches_the_command(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/parity.feature",
    name = "A rejected answer runs nothing"
)]
#[test]
fn a_rejected_answer_runs_nothing(world: World) {
    let _ = world;
}
