//! A command tree declared where its commands live (decision-14, CLI-32).
//!
//! The tree here is three levels deep — `cast db migrate` — because depth is the thing a
//! tree has to get right and a two-level example cannot show: the inner `db` node is an
//! ordinary leaf struct that itself declares `subcommands`, and nothing in the macros
//! knows how deep it sits.
//!
//! What is proven, in order: clap still parses and renders help with the full path; the
//! conversion to typed input works at every node; an absent optional subcommand reaches
//! the application; and the generated `Command`, `Planned` and `Collect` impls delegate to
//! the chosen leaf, with the leaf's own `collect = path` honoured.

use std::cell::RefCell;

use clap::{CommandFactory, FromArgMatches, Parser};
use clifacet::command::{Plan, Planned};
use clifacet::policy::{Capabilities, Mode, Policy, Request as ModeRequest};
use clifacet::prompt::{Answer, Collect, Scripted, Session};
use clifacet::{Command, Facet, Failure, FailureKind};

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
    #[clifacet(alias = "hi")]
    Greet(Greet),
    /// Work on the database.
    Db(Db),
    /// Nothing to see.
    #[clifacet(name = "secret-handshake", hide)]
    Handshake(Handshake),
}

#[derive(Debug, PartialEq)]
enum Request {
    Greet(GreetInput),
    Db(DbInput),
    Handshake(HandshakeInput),
}

/// What every command here returns, so the tree has one output type.
#[derive(Debug, PartialEq)]
struct Said(String);

#[clifacet::command(input = GreetInput)]
#[derive(Clone)]
struct Greet {
    #[clifacet(long, ask = "Who should I greet?", fill = "world", trim)]
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

#[clifacet::subcommands(input = DbRequest, output = Said, planned)]
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

/// Its questions are its own: `collect = path` replaces the generated ones.
#[clifacet::command(input = ResetInput, collect = confirm_reset)]
#[derive(Clone)]
struct Reset {
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

#[clifacet::command(input = HandshakeInput)]
#[derive(Clone)]
struct Handshake {}

#[derive(Debug, PartialEq)]
struct HandshakeInput {}

// --- the leaves as commands -----------------------------------------------------

impl Command for Greet {
    type Input = GreetInput;
    type Output = Said;

    fn execute(&self, input: GreetInput) -> Result<Said, Failure> {
        Ok(Said(format!("hello, {}", input.name)))
    }
}

impl Command for Handshake {
    type Input = HandshakeInput;
    type Output = Said;

    fn execute(&self, _input: HandshakeInput) -> Result<Said, Failure> {
        Ok(Said(String::from("*handshake*")))
    }
}

/// The inner node runs its own subtree; the input carries the node's own flag along.
impl Command for Db {
    type Input = DbInput;
    type Output = Said;

    fn execute(&self, input: DbInput) -> Result<Said, Failure> {
        let Said(done) = self.command.execute(input.command)?;
        Ok(Said(format!("{} on {}", done, input.url)))
    }
}

/// A leaf output that is not the tree's: the generated arm converts it with `Into`.
struct Migrated(u8);

impl From<Migrated> for Said {
    fn from(Migrated(count): Migrated) -> Self {
        Self(format!("migrated {count}"))
    }
}

thread_local! {
    static RAN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn ran(what: &str) {
    RAN.with(|log| log.borrow_mut().push(what.to_owned()));
}

fn take_ran() -> Vec<String> {
    RAN.with(|log| log.borrow_mut().drain(..).collect())
}

impl Planned for Migrate {
    type Input = MigrateInput;
    type Output = Migrated;

    fn plan(&self, input: MigrateInput) -> Result<Plan<'_, Migrated>, Failure> {
        let steps = input.steps.unwrap_or(3);
        Ok(Plan::new(move || {
            ran("migrate");
            Ok(Migrated(steps))
        })
        .step(format!("apply {steps} migrations")))
    }
}

impl Planned for Reset {
    type Input = ResetInput;
    type Output = Said;

    fn plan(&self, input: ResetInput) -> Result<Plan<'_, Said>, Failure> {
        if !input.force {
            return Err(Failure::cancelled("nothing was dropped"));
        }
        Ok(Plan::new(|| {
            ran("reset");
            Ok(Said(String::from("reset")))
        })
        .step("drop every table"))
    }
}

// --- helpers --------------------------------------------------------------------

fn parse(arguments: &[&str]) -> Cast {
    Cast::try_parse_from(std::iter::once("cast").chain(arguments.iter().copied()))
        .expect("the tree parses its own command line")
}

/// What clap says about a command line it does not accept.
fn rejected(arguments: &[&str]) -> clap::Error {
    match Cast::try_parse_from(std::iter::once("cast").chain(arguments.iter().copied())) {
        Ok(_) => panic!("clap accepted {arguments:?}"),
        Err(error) => error,
    }
}

fn chosen(cast: Cast) -> Commands {
    cast.command.expect("a subcommand was given")
}

/// The tree to run, and the request validated from the same parse: validation consumes
/// the parsed command, so the tree that runs is a clone of it.
fn tree_and_request(arguments: &[&str]) -> (Commands, Request) {
    let cast = parse(arguments);
    let tree = cast.command.clone().expect("a subcommand was given");
    let request = cast.validate().expect("valid").command.expect("converted");
    (tree, request)
}

fn session(answers: impl IntoIterator<Item = Answer>) -> Session {
    let plan = Policy::up_to(Mode::Interactive)
        .resolve(ModeRequest::none(), Capabilities::terminal())
        .expect("prompts are permitted at a terminal");
    let script = Scripted::new(answers);
    Session::start(plan, move || Box::new(script))
}

// --- clap still parses, three levels down ---------------------------------------

#[test]
fn a_leaf_three_levels_deep_parses_into_typed_values() {
    let input = parse(&["db", "--url", "staging", "migrate", "--steps", "2"])
        .validate()
        .expect("valid");

    assert_eq!(
        input,
        CastInput {
            verbose: false,
            command: Some(Request::Db(DbInput {
                url: String::from("staging"),
                command: DbRequest::Migrate(MigrateInput { steps: Some(2) }),
            })),
        }
    );
}

#[test]
fn the_help_of_a_deep_leaf_shows_its_full_path() {
    let error = rejected(&["db", "migrate", "--help"]);
    let help = error.to_string();

    assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(help.contains("Usage: cast db migrate"), "{help}");
    assert!(help.contains("Stop after this many"), "{help}");
}

#[test]
fn a_global_root_flag_is_accepted_at_the_leaf() {
    let input = parse(&["db", "migrate", "--verbose"])
        .validate()
        .expect("valid");

    assert!(input.verbose);
}

#[test]
fn clap_rejects_what_the_grammar_does_not_allow_before_anything_converts() {
    let error = rejected(&["db", "migrate", "--steps", "300"]);

    assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
}

// --- the conversion works at every node -----------------------------------------

#[test]
fn every_node_converts_from_its_own_matches() {
    let root = <Cast as CommandFactory>::command();
    let matches = root
        .try_get_matches_from(["cast", "db", "--url", "x", "migrate", "--steps", "1"])
        .expect("parses");

    // The tree under the root, from the root's matches.
    let commands = Commands::from_arg_matches(&matches).expect("the tree");
    assert_eq!(
        Request::try_from(commands).expect("valid"),
        Request::Db(DbInput {
            url: String::from("x"),
            command: DbRequest::Migrate(MigrateInput { steps: Some(1) }),
        })
    );

    // The inner node, from its own subcommand's matches.
    let (name, db_matches) = matches.subcommand().expect("db");
    assert_eq!(name, "db");
    let db = Db::from_arg_matches(db_matches).expect("the db node");
    assert_eq!(db.validate().expect("valid").url, "x");

    // The inner tree and the leaf below it.
    let inner = DbCommands::from_arg_matches(db_matches).expect("the inner tree");
    assert_eq!(
        DbRequest::try_from(inner).expect("valid"),
        DbRequest::Migrate(MigrateInput { steps: Some(1) })
    );
    let (_, leaf_matches) = db_matches.subcommand().expect("migrate");
    let leaf = Migrate::from_arg_matches(leaf_matches).expect("the leaf");
    assert_eq!(
        leaf.validate().expect("valid"),
        MigrateInput { steps: Some(1) }
    );
}

#[test]
fn a_leaf_validates_by_its_own_rules_inside_the_tree() {
    let input = parse(&["greet", "--name", "  ada  "])
        .validate()
        .expect("valid");

    assert_eq!(
        input.command,
        Some(Request::Greet(GreetInput {
            name: String::from("ada")
        }))
    );
    let filled = parse(&["greet"]).validate().expect("valid");
    assert_eq!(
        filled.command,
        Some(Request::Greet(GreetInput {
            name: String::from("world")
        }))
    );
}

// --- an optional subcommand reaches the application ------------------------------

#[test]
fn an_absent_optional_subcommand_reaches_the_application_as_none() {
    let input = parse(&["--verbose"]).validate().expect("valid");

    assert_eq!(
        input,
        CastInput {
            verbose: true,
            command: None
        }
    );
}

#[test]
fn a_required_inner_subcommand_keeps_clap_bare_invocation_verdict() {
    let error = rejected(&["db"]);

    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    );
    assert!(
        error.use_stderr(),
        "a bare node is a usage outcome, not help on stdout"
    );
}

// --- variant keys -----------------------------------------------------------------

#[test]
fn a_variant_alias_and_name_reach_clap() {
    let by_alias = chosen(parse(&["hi", "--name", "bo"]));
    assert!(matches!(by_alias, Commands::Greet(_)));

    let by_name = chosen(parse(&["secret-handshake"]));
    assert!(matches!(by_name, Commands::Handshake(_)));
    rejected(&["handshake"]);
}

#[test]
fn a_hidden_variant_is_left_out_of_the_help_and_the_variant_doc_is_its_about() {
    let help = <Cast as CommandFactory>::command()
        .render_help()
        .to_string();

    assert!(!help.contains("secret-handshake"), "{help}");
    // clap drops a one-line doc comment's closing period, as it does for any about.
    assert!(help.contains("greet  Say hello"), "{help}");
    assert!(help.contains("db     Work on the database"), "{help}");
}

// --- the generated Command delegates to the chosen leaf ---------------------------

#[test]
fn the_tree_runs_the_chosen_leaf_and_converts_its_output() {
    let (tree, request) = tree_and_request(&["greet", "--name", "ada"]);

    assert_eq!(
        tree.execute(request).expect("runs"),
        Said(String::from("hello, ada"))
    );

    let (tree, request) = tree_and_request(&["db", "migrate", "--steps", "4"]);
    take_ran();
    assert_eq!(
        tree.execute(request).expect("runs"),
        Said(String::from("migrated 4 on main"))
    );
    assert_eq!(
        take_ran(),
        ["migrate"],
        "a planned leaf runs through its plan"
    );
}

#[test]
fn an_input_for_another_subcommand_is_an_execution_failure_not_a_panic() {
    let (tree, _) = tree_and_request(&["greet"]);
    let other = Request::Handshake(HandshakeInput {});

    let failure = tree.execute(other).expect_err("mismatched");

    assert_eq!(failure.kind(), FailureKind::Execution);
}

// --- the generated Planned delegates to the chosen leaf ---------------------------

#[test]
fn a_planned_tree_describes_the_chosen_leaf_and_executes_that_plan() {
    let tree = DbCommands::Migrate(Migrate { steps: Some(2) });

    let plan = tree
        .plan(DbRequest::Migrate(MigrateInput { steps: Some(2) }))
        .expect("plans");
    take_ran();

    assert_eq!(plan.describe(), "apply 2 migrations");
    assert_eq!(take_ran(), [""; 0], "describing runs nothing");
    assert_eq!(
        plan.execute().expect("runs"),
        Said(String::from("migrated 2"))
    );
    assert_eq!(take_ran(), ["migrate"]);
}

#[test]
fn a_leaf_that_refuses_to_plan_refuses_through_the_tree() {
    let tree = DbCommands::Reset(Reset { force: false });

    let failure = tree
        .plan(DbRequest::Reset(ResetInput { force: false }))
        .expect_err("refuses");

    assert_eq!(failure.kind(), FailureKind::Cancelled);
}

#[test]
fn a_plan_maps_its_output_and_keeps_its_steps() {
    let plan = Plan::new(|| Ok(2_u8)).step("count").map(|count| count * 10);

    assert_eq!(plan.steps(), ["count"]);
    assert_eq!(plan.execute().expect("runs"), 20);
}

// --- the generated Collect delegates to the chosen leaf ---------------------------

#[test]
fn a_question_declared_on_a_leaf_is_asked_through_the_tree() {
    let mut session = session([Answer::Text(String::from("grace"))]);

    let input = parse(&["greet"])
        .collect(&mut session)
        .and_then(Facet::validate)
        .expect("valid");

    assert_eq!(
        input.command,
        Some(Request::Greet(GreetInput {
            name: String::from("grace")
        }))
    );
}

#[test]
fn a_leaf_collect_path_replaces_its_generated_questions_at_depth_two() {
    let mut session = session([Answer::Yes]);

    let input = parse(&["db", "reset"])
        .collect(&mut session)
        .and_then(Facet::validate)
        .expect("valid");

    assert_eq!(
        input.command,
        Some(Request::Db(DbInput {
            url: String::from("main"),
            command: DbRequest::Reset(ResetInput { force: true }),
        }))
    );
}

#[test]
fn an_absent_subcommand_asks_nothing() {
    let mut session = session([]);

    let input = parse(&[])
        .collect(&mut session)
        .and_then(Facet::validate)
        .expect("valid");

    assert_eq!(input.command, None);
}
