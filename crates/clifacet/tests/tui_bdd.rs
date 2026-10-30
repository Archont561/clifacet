//! The full-screen form's user-visible promise, driven without a terminal.
//!
//! `example::run_tui` takes its screen and its keystrokes as parameters, so these
//! scenarios exercise the production path — enter the alternate screen, edit the
//! draft, submit or cancel, leave the screen — against a buffer and a script.

#![cfg(feature = "tui")]

use clifacet::example;
use clifacet::tui::Key;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// What one TUI run produced.
#[derive(Default)]
struct World {
    /// The questions the screen asks, which is what "the form is available" means.
    questions: Vec<String>,
    keys: Vec<Key>,
    code: i32,
    data: String,
    diagnostic: String,
}

#[fixture]
fn world() -> World {
    World::default()
}

#[given("the example's full-screen form")]
fn the_form_is_available(world: &mut World) {
    world.questions = example::tui_form()
        .fields()
        .iter()
        .map(|field| field.name().to_owned())
        .collect();

    assert_eq!(
        world.questions,
        ["name", "repeat", "level"],
        "the form asks for the values and not for the flags that chose the mode"
    );
}

#[when("the user types {text} and submits")]
fn the_user_types_and_submits(world: &mut World, text: String) {
    world.keys.extend(text.chars().map(Key::Char));
    world.keys.push(Key::Enter);
    world.run();
}

#[when("the user presses escape")]
fn the_user_presses_escape(world: &mut World) {
    world.keys.push(Key::Escape);
    world.run();
}

#[then("the data is what --name {name} would produce")]
fn the_data_matches_the_flag(world: &World, name: String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::main_with(
        [
            String::from("example"),
            String::from("--name"),
            name.clone(),
        ],
        clifacet::policy::Capabilities::headless(),
        || Box::new(clifacet::prompt::Scripted::default()),
        &mut out,
        &mut err,
    );
    let expected = String::from_utf8_lossy(&out);

    assert_eq!(
        world.data, expected,
        "the same name must produce the same data"
    );
    assert_eq!(world.code, code);
}

#[then("the exit code says the run succeeded")]
fn the_run_succeeded(world: &World) {
    assert_eq!(world.code, example::EXIT_SUCCESS);
}

#[then("nothing is written to the data stream")]
fn nothing_was_written(world: &World) {
    assert!(world.data.is_empty(), "a cancelled run writes no data");
}

#[then("the exit code says the run was cancelled")]
fn the_run_was_cancelled(world: &World) {
    assert_eq!(world.code, example::EXIT_CANCELLED);
    assert!(
        world.diagnostic.contains("cancel"),
        "the diagnostic explains the exit: {}",
        world.diagnostic
    );
}

impl World {
    /// Runs the TUI with the collected keys against an in-memory screen.
    fn run(&mut self) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        self.code = example::run_tui(Vec::new(), self.keys.iter().copied(), &mut out, &mut err);
        self.data = String::from_utf8_lossy(&out).into_owned();
        self.diagnostic = String::from_utf8_lossy(&err).into_owned();
    }
}

#[scenario(
    path = "tests/features/tui.feature",
    name = "A typed answer produces the same data as the flag"
)]
#[test]
fn a_typed_answer_matches_the_flag(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tui.feature",
    name = "Leaving the form runs nothing"
)]
#[test]
fn leaving_the_form_runs_nothing(world: World) {
    let _ = world;
}
