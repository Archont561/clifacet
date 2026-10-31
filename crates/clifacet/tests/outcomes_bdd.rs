//! The stream and exit contract, stated as scenarios.
//!
//! Each step drives the example's public entry point with explicit capabilities, so
//! what the scenarios assert is what a piped run would see.

use std::cell::RefCell;
use std::io::{Cursor, Write};
use std::rc::Rc;

use clifacet::example;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted, Terminal};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// A writer that keeps what a frontend wrote, so a scenario can assert about it.
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

/// What one scenario has collected.
struct World {
    capabilities: Capabilities,
    frontend: Option<Box<dyn Prompter>>,
    /// What the frontend wrote, kept apart from the two streams under test.
    transcript: Shared,
    argv: Vec<String>,
    code: i32,
    data: String,
    diagnostic: String,
}

impl Default for World {
    fn default() -> Self {
        Self {
            capabilities: Capabilities::headless(),
            frontend: None,
            transcript: Shared::default(),
            argv: vec![String::from("example")],
            code: 0,
            data: String::new(),
            diagnostic: String::new(),
        }
    }
}

#[fixture]
fn world() -> World {
    World::default()
}

#[given("the example can run without a terminal")]
fn headless(world: &mut World) {
    world.capabilities = Capabilities::headless();
}

#[given("the example may ask a question")]
fn asks_the_user(world: &mut World) {
    world.capabilities = Capabilities::terminal();
    // A real `Terminal`, not a script: this scenario is about where a question goes,
    // and a scripted frontend answers without writing anything.
    world.frontend = Some(Box::new(Terminal::new(
        Cursor::new("ada\n"),
        world.transcript.clone(),
    )));
}

#[when("the example greets ada")]
fn greets_ada(world: &mut World) {
    world
        .argv
        .extend([String::from("--name"), String::from("ada")]);
    world.run();
}

#[when("the answer is ada")]
fn answer_is_ada(world: &mut World) {
    world.run();
}

#[when("the repeat count is out of range")]
fn repeat_out_of_range(world: &mut World) {
    world
        .argv
        .extend([String::from("--repeat"), String::from("9")]);
    world.run();
}

#[then("the data stream is exactly the greeting")]
fn data_is_the_greeting(world: &World) {
    assert_eq!(world.data, format!("hello, {}\n", World::NAME));
}

#[then("the diagnostic stream is empty")]
fn diagnostic_is_empty(world: &World) {
    assert_eq!(world.diagnostic, "");
}

#[then("the exit code is the success code")]
fn exit_is_success(world: &World) {
    assert_eq!(world.code, example::EXIT_SUCCESS);
}

#[then("the question is on the diagnostic stream")]
fn question_is_a_diagnostic(world: &World) {
    let asked = String::from_utf8_lossy(&world.transcript.0.borrow()).into_owned();

    assert!(
        asked.contains("Who should I greet?"),
        "the question a person answers is not data: {asked:?}"
    );
    assert!(
        !world.data.contains("Who should I greet?"),
        "and it is not on the data stream either"
    );
    assert_eq!(world.code, example::EXIT_SUCCESS);
}

#[then("the data stream is empty")]
fn data_is_empty(world: &World) {
    assert_eq!(world.data, "");
}

#[then("the exit code is the validation code")]
fn exit_is_validation(world: &World) {
    assert_eq!(world.code, example::EXIT_VALIDATION_FAILED);
    assert!(
        world
            .diagnostic
            .contains("--repeat must be between 1 and 8")
    );
}

impl World {
    /// The name the scenarios answer with.
    const NAME: &'static str = "ada";

    /// Runs the example with the collected capabilities and stream sinks.
    fn run(&mut self) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let frontend = self
            .frontend
            .take()
            .unwrap_or_else(|| Box::new(Scripted::default()) as Box<dyn Prompter>);

        self.code = example::main_with(
            self.argv.iter().cloned(),
            self.capabilities,
            move || frontend,
            &mut out,
            &mut err,
        );
        self.data = String::from_utf8_lossy(&out).into_owned();
        self.diagnostic = String::from_utf8_lossy(&err).into_owned();
    }
}

#[scenario(
    path = "tests/features/outcomes.feature",
    name = "A successful run writes data and nothing else"
)]
#[test]
fn a_successful_run(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/outcomes.feature",
    name = "A question is never data"
)]
#[test]
fn a_question_is_never_data(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/outcomes.feature",
    name = "A failure writes a diagnostic and no data"
)]
#[test]
fn a_failure_writes_a_diagnostic(world: World) {
    let _ = world;
}
