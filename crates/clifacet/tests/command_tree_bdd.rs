//! A command tree, as a person running it sees it (decision-14, CLI-32).
//!
//! The tree is three levels deep — `cast db migrate` — and every scenario runs it the way
//! an application does: clap parses the line, the parsed tree asks its questions on a
//! scripted session, `Facet::validate` turns it into the typed request, and the tree's
//! generated `Command` runs the chosen leaf. No application sits in between; these are the
//! library's own promises. `command_tree.rs` proves the mechanics case by case.

use std::cell::RefCell;

use clap::Parser as _;
use clifacet::policy::{Capabilities, Mode, Policy, Request as ModeRequest};
use clifacet::prompt::{Answer, Collect, Scripted, Session};
use clifacet::{Command, Facet, Failure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

// --- the tree -------------------------------------------------------------------

/// The root: a flag of its own, and the tree.
#[clifacet::command(input = CastInput)]
#[command(name = "cast")]
struct Cast {
    /// Talk more.
    #[clifacet(long)]
    #[arg(global = true)]
    verbose: bool,

    #[clifacet(subcommands)]
    command: Option<Commands>,
}

#[derive(Debug, PartialEq)]
struct CastInput {
    verbose: bool,
    command: Option<Request>,
}

#[clifacet::subcommands(input = Request, output = Said)]
#[derive(Clone)]
enum Commands {
    /// Say hello.
    Greet(Greet),
    /// Work on the database.
    Db(Db),
}

#[derive(Debug, PartialEq)]
enum Request {
    Greet(GreetInput),
    Db(DbInput),
}

/// What every command here answers, so the tree has one output type.
#[derive(Debug, PartialEq)]
struct Said(String);

#[clifacet::command(input = GreetInput)]
#[derive(Clone)]
struct Greet {
    /// Who to greet.
    #[clifacet(long, ask = "Who should I greet?", fill = "world")]
    name: Option<String>,
}

#[derive(Debug, PartialEq)]
struct GreetInput {
    name: String,
}

/// An inner node: a leaf to the tree above, a tree to the one below.
#[clifacet::command(input = DbInput)]
#[derive(Clone)]
struct Db {
    /// Which database.
    #[clifacet(long, default_value = "main")]
    url: String,

    #[clifacet(subcommands)]
    command: DbCommands,
}

#[derive(Debug, PartialEq)]
struct DbInput {
    url: String,
    command: DbRequest,
}

#[clifacet::subcommands(input = DbRequest, output = Said)]
#[derive(Clone)]
enum DbCommands {
    /// Apply the pending migrations.
    Migrate(Migrate),
    /// Drop everything.
    Reset(Reset),
}

#[derive(Debug, PartialEq)]
enum DbRequest {
    Migrate(MigrateInput),
    Reset(ResetInput),
}

#[clifacet::command(input = MigrateInput)]
#[derive(Clone)]
struct Migrate {
    /// Stop after this many.
    #[clifacet(long)]
    steps: Option<u8>,
}

#[derive(Debug, PartialEq)]
struct MigrateInput {
    steps: Option<u8>,
}

/// Its question is its own: `collect = path` replaces the generated ones.
#[clifacet::command(input = ResetInput, collect = confirm_reset)]
#[derive(Clone)]
struct Reset {
    /// Skip the question.
    #[clifacet(long)]
    force: bool,
}

#[derive(Debug, PartialEq)]
struct ResetInput {
    force: bool,
}

fn confirm_reset(mut reset: Reset, session: &mut Session) -> Result<Reset, Failure> {
    if !reset.force {
        reset.force = session.confirm("Drop every table?")?.is_confirmed();
    }
    Ok(reset)
}

// --- the leaves as commands -----------------------------------------------------

thread_local! {
    /// The database commands that actually ran, in order.
    static RAN: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

impl Command for Greet {
    type Input = GreetInput;
    type Output = Said;

    fn execute(&self, input: GreetInput) -> Result<Said, Failure> {
        Ok(Said(format!("hello, {}", input.name)))
    }
}

impl Command for Db {
    type Input = DbInput;
    type Output = Said;

    fn execute(&self, input: DbInput) -> Result<Said, Failure> {
        self.command.execute(input.command)
    }
}

impl Command for Migrate {
    type Input = MigrateInput;
    type Output = Said;

    fn execute(&self, input: MigrateInput) -> Result<Said, Failure> {
        RAN.with(|ran| ran.borrow_mut().push("migrate"));
        Ok(Said(format!("migrated {}", input.steps.unwrap_or(1))))
    }
}

impl Command for Reset {
    type Input = ResetInput;
    type Output = Said;

    fn execute(&self, _input: ResetInput) -> Result<Said, Failure> {
        RAN.with(|ran| ran.borrow_mut().push("reset"));
        Ok(Said(String::from("reset")))
    }
}

// --- the world ------------------------------------------------------------------

/// One run of `cast`, and what it showed.
struct World {
    capabilities: Capabilities,
    input: Option<Result<CastInput, Failure>>,
    said: Option<Result<Said, Failure>>,
    asked: Vec<String>,
}

impl World {
    /// Parses `line`, then asks and validates the way an application does.
    ///
    /// Returns the tree as collected — before validation consumes it — so a scenario
    /// that runs it runs exactly what was asked about.
    fn collect(&mut self, line: &str, answers: Vec<Answer>) -> Option<Commands> {
        let cast =
            Cast::try_parse_from(line.split_whitespace()).expect("the tree parses its own line");
        let requested = if self.capabilities.supports(Mode::Interactive) {
            ModeRequest::mode(Mode::Interactive)
        } else {
            ModeRequest::none()
        };
        let plan = Policy::up_to(Mode::Interactive)
            .resolve(requested, self.capabilities)
            .expect("the mode is permitted");
        let script = Scripted::new(answers);
        let probe = script.clone();
        let mut session = Session::start(plan, move || Box::new(probe));
        let collected = cast.collect(&mut session);
        self.asked = script.asked();
        let tree = collected
            .as_ref()
            .ok()
            .and_then(|cast| cast.command.clone());
        self.input = Some(collected.and_then(Facet::validate));
        tree
    }

    fn input(&self) -> &CastInput {
        match &self.input {
            Some(Ok(input)) => input,
            other => panic!("the run did not validate: {other:?}"),
        }
    }
}

#[fixture]
fn world() -> World {
    RAN.with(|ran| ran.borrow_mut().clear());
    World {
        capabilities: Capabilities::headless(),
        input: None,
        said: None,
        asked: Vec::new(),
    }
}

// --- steps ----------------------------------------------------------------------

#[given("the tree runs at a terminal")]
fn at_a_terminal(world: &mut World) {
    world.capabilities = Capabilities::terminal();
}

#[when("the user runs {line:string}")]
fn the_user_runs(world: &mut World, line: String) {
    world.collect(&line, Vec::new());
}

#[when("the user runs {line:string} and confirms")]
fn the_user_runs_and_confirms(world: &mut World, line: String) {
    world.collect(&line, vec![Answer::Yes]);
}

#[when("the user runs {line:string} through the tree")]
fn the_user_runs_through_the_tree(world: &mut World, line: String) {
    let tree = world
        .collect(&line, Vec::new())
        .expect("a subcommand was given");
    let request = match world.input.take() {
        Some(Ok(input)) => input.command.expect("the request carries the command"),
        other => panic!("the run did not validate: {other:?}"),
    };
    world.said = Some(tree.execute(request));
}

#[then("the application receives the root flag, the replica database and 2 steps")]
fn receives_every_level(world: &World) {
    assert_eq!(
        world.input(),
        &CastInput {
            verbose: true,
            command: Some(Request::Db(DbInput {
                url: String::from("replica"),
                command: DbRequest::Migrate(MigrateInput { steps: Some(2) }),
            })),
        }
    );
}

#[then("the only question asked was {question:string}")]
fn the_only_question_was(world: &World, question: String) {
    assert_eq!(world.asked, [question]);
}

#[then("the reset is forced")]
fn the_reset_is_forced(world: &World) {
    assert_eq!(
        world.input().command,
        Some(Request::Db(DbInput {
            url: String::from("main"),
            command: DbRequest::Reset(ResetInput { force: true }),
        }))
    );
}

#[then("the tree answers {answer:string}")]
fn the_tree_answers(world: &World, answer: String) {
    match &world.said {
        Some(Ok(said)) => assert_eq!(said, &Said(answer)),
        other => panic!("the tree did not run: {other:?}"),
    }
}

#[then("no database command ran")]
fn no_database_command_ran(world: &World) {
    let _ = world;
    RAN.with(|ran| assert!(ran.borrow().is_empty(), "ran: {:?}", ran.borrow()));
}

#[then("the application receives no command")]
fn receives_no_command(world: &World) {
    assert_eq!(world.input().command, None);
}

#[then("nothing was asked")]
fn nothing_was_asked(world: &World) {
    assert!(world.asked.is_empty(), "asked: {:?}", world.asked);
}

// --- scenarios ------------------------------------------------------------------

#[scenario(
    path = "features/command_tree.feature",
    name = "Every level's values reach the chosen leaf"
)]
#[test]
fn every_level_reaches_the_leaf(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/command_tree.feature",
    name = "Only the chosen leaf asks its own question"
)]
#[test]
fn only_the_chosen_leaf_asks(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/command_tree.feature",
    name = "Running the tree runs only the chosen leaf"
)]
#[test]
fn only_the_chosen_leaf_runs(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/command_tree.feature",
    name = "Without a subcommand the application decides"
)]
#[test]
fn without_a_subcommand(world: World) {
    let _ = world;
}
