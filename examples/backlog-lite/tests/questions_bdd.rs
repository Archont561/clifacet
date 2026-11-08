//! When `bl` asks, and what it does when it cannot, as executable scenarios.
//!
//! Every scenario drives `app` — the entry point the binary uses — with `--dry-run`, so
//! the planned command is printed instead of spawned, and with a scripted frontend, so a
//! scenario that asks something unexpected fails instead of waiting for a person.

use std::convert::Infallible;

use clap::Parser as _;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Scripted};
use clifacet_backlog_lite::{Cli, EXIT, Program, Status, app};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// One run of `bl`, and what it showed.
struct World {
    capabilities: Capabilities,
    code: i32,
    out: String,
    err: String,
    asked: Vec<String>,
}

impl World {
    /// Runs `bl --dry-run …` with the scripted answers.
    fn run(&mut self, arguments: &[&str], answers: impl IntoIterator<Item = Answer>) {
        let cli = Cli::try_parse_from(
            ["bl", "--dry-run"]
                .into_iter()
                .chain(arguments.iter().copied()),
        )
        .expect("bl parses its own command line");
        // `--dry-run` arrives in the globals, which the runner takes apart from the `Cli`.
        let globals = cli.globals;
        let script = Scripted::new(answers);
        let probe = script.clone();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        self.code = app(|| Ok::<_, Infallible>(Program::default())).run_parsed(
            cli,
            globals,
            self.capabilities,
            move || Box::new(probe),
            &mut out,
            &mut err,
        );
        self.out = String::from_utf8(out).expect("UTF-8");
        self.err = String::from_utf8(err).expect("UTF-8");
        self.asked = script.asked();
    }
}

#[fixture]
fn world() -> World {
    World {
        capabilities: Capabilities::headless(),
        code: -1,
        out: String::new(),
        err: String::new(),
        asked: Vec::new(),
    }
}

#[given("bl runs at a terminal")]
fn at_a_terminal(world: &mut World) {
    world.capabilities = Capabilities::terminal();
}

#[given("bl runs without a terminal")]
fn without_a_terminal(world: &mut World) {
    world.capabilities = Capabilities::headless();
}

#[when("the user lists with a bare --status and picks In Progress")]
fn lists_and_picks(world: &mut World) {
    world.run(
        &["list", "--status"],
        [Answer::Text(String::from("In Progress"))],
    );
}

#[when("the user lists with a bare --status")]
fn lists_with_a_bare_status(world: &mut World) {
    world.run(&["list", "--status"], []);
}

#[when("the user marks CLI-20 done with --yes")]
fn marks_done_with_yes(world: &mut World) {
    world.run(&["--yes", "done", "CLI-20"], []);
}

#[when("the user marks CLI-20 done")]
fn marks_done(world: &mut World) {
    world.run(&["done", "CLI-20"], []);
}

#[when("the user marks CLI-20 done and declines")]
fn marks_done_and_declines(world: &mut World) {
    world.run(&["done", "CLI-20"], [Answer::No]);
}

#[then("the planned command filters by In Progress")]
fn filters_by_in_progress(world: &World) {
    assert_eq!(world.code, EXIT.success(), "{}", world.err);
    assert_eq!(world.asked, ["Which status?"]);
    assert!(
        world
            .out
            .contains("backlog task list --status 'In Progress'"),
        "{}",
        world.out
    );
}

#[then("the run is refused as incomplete, naming every status")]
fn refused_naming_every_status(world: &World) {
    assert_eq!(world.code, EXIT.validation());
    assert!(world.out.is_empty(), "nothing was planned: {}", world.out);
    for status in Status::ALL {
        assert!(world.err.contains(status.as_str()), "{}", world.err);
    }
}

#[then("the planned command moves CLI-20 to Done")]
fn moves_to_done(world: &World) {
    assert_eq!(world.code, EXIT.success(), "{}", world.err);
    assert_eq!(world.asked, [""; 0].map(String::from), "--yes answered it");
    assert!(
        world.out.contains("backlog task edit CLI-20 --status Done"),
        "{}",
        world.out
    );
}

#[then("the run is refused, saying to pass --yes")]
fn refused_saying_yes(world: &World) {
    assert_eq!(world.code, EXIT.validation());
    assert!(world.out.is_empty(), "nothing was planned: {}", world.out);
    assert!(world.err.contains("--yes"), "{}", world.err);
}

#[then("nothing is planned and the run is cancelled")]
fn nothing_planned_and_cancelled(world: &World) {
    assert_eq!(world.code, EXIT.cancelled());
    assert!(world.out.is_empty(), "nothing was planned: {}", world.out);
    assert_eq!(world.asked, ["Move CLI-20 to Done?"]);
}

#[scenario(
    path = "features/questions.feature",
    name = "A bare status filter is chosen from a list"
)]
#[test]
fn a_bare_status_filter_is_chosen(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/questions.feature",
    name = "A bare status filter nothing can ask about is refused"
)]
#[test]
fn a_bare_status_filter_is_refused_headless(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/questions.feature",
    name = "--yes moves a task even without a terminal"
)]
#[test]
fn yes_moves_a_task_headless(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/questions.feature",
    name = "A move nobody could confirm asks for --yes"
)]
#[test]
fn an_unconfirmable_move_asks_for_yes(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/questions.feature",
    name = "A declined move cancels the run"
)]
#[test]
fn a_declined_move_cancels(world: World) {
    let _ = world;
}
