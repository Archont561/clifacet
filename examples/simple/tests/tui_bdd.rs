//! The full-screen form's user-visible promise, driven without a terminal.
//!
//! `example::run_tui` takes its screen and its keystrokes as parameters, so these
//! scenarios exercise the production path — enter the alternate screen, edit the
//! draft, submit or cancel, leave the screen — against a buffer and a script.

use clifacet::tui::Key;
use clifacet_example as example;
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

#[when("the user types {text}, presses {letter} on the level and submits")]
fn the_user_chooses_a_level(world: &mut World, text: String, letter: char) {
    world.keys.extend(text.chars().map(Key::Char));
    world
        .keys
        .extend([Key::Tab, Key::Tab, Key::Char(letter), Key::Enter]);
    world.run();
}

#[then("the level is offered as a choice of {first} and {second}")]
fn the_level_is_a_choice(first: String, second: String) {
    let form = example::tui_form();
    let level = form.field("level").expect("the form asks for the level");

    assert_eq!(level.choices(), [first, second]);
    assert_eq!(
        level.value(),
        "basic",
        "the default is chosen to begin with"
    );
}

#[when("the user presses escape")]
fn the_user_presses_escape(world: &mut World) {
    world.keys.push(Key::Escape);
    world.run();
}

#[then("the data is what {flags} would produce")]
fn the_data_matches_the_flags(world: &World, flags: String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::main_with(
        std::iter::once("example")
            .chain(flags.split_whitespace())
            .map(String::from),
        clifacet::policy::Capabilities::headless(),
        || Box::new(clifacet::prompt::Scripted::default()),
        &mut out,
        &mut err,
    );
    let expected = String::from_utf8_lossy(&out);

    assert_eq!(
        world.data, expected,
        "the same answers must produce the same data"
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
    path = "features/tui.feature",
    name = "A typed answer produces the same data as the flag"
)]
#[test]
fn a_typed_answer_matches_the_flag(world: World) {
    let _ = world;
}

#[scenario(path = "features/tui.feature", name = "Leaving the form runs nothing")]
#[test]
fn leaving_the_form_runs_nothing(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/tui.feature",
    name = "Choosing a level produces the same data as the flag"
)]
#[test]
fn a_chosen_level_matches_the_flag(world: World) {
    let _ = world;
}
