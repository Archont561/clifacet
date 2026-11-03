//! Executable description of the companion derive's promise.
//!
//! The steps drive the same public seam a consumer uses — `clap` parses, then
//! `Facet::validate` bridges to typed input — so the scenario fails if the
//! derive ever needs a private clap API or a second grammar to work.

use clifacet::clap::Parser as _;
use clifacet::{Command, Facet};
use clifacet_example::{Cli, Example, ExampleInput};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// State shared between the steps of one scenario.
#[derive(Default)]
struct AppWorld {
    alias: String,
    input: Option<ExampleInput>,
    help: String,
}

#[fixture]
fn world() -> AppWorld {
    AppWorld::default()
}

#[given("an application whose name flag also answers to {alias}")]
fn the_application_has_an_alias(world: &mut AppWorld, alias: String) {
    // Read the alias out of clap's own command: if the derive invented a grammar,
    // this is where the two would diverge.
    let command = <Cli as Facet>::command();
    let name = command
        .get_arguments()
        .find(|argument| argument.get_id() == "name")
        .expect("the example command declares a name argument");
    assert!(
        name.get_all_aliases()
            .unwrap_or_default()
            .contains(&alias.as_str()),
        "clap does not report {alias:?} as an alias of --name"
    );
    world.alias = alias;
}

#[when("the user runs the command with --{flag} and {value}")]
fn the_user_runs_with_the_alias(world: &mut AppWorld, flag: String, value: String) {
    assert_eq!(
        flag, world.alias,
        "the scenario drives the alias it declared"
    );
    let cli = Cli::try_parse_from(["example", &format!("--{flag}"), &value])
        .expect("an alias parses through clap");
    world.input = Some(cli.validate().expect("a valid alias value is valid input"));
}

#[then("the typed input carries the name {name:string}")]
fn the_input_carries_the_name(world: &mut AppWorld, name: String) {
    let input = world
        .input
        .as_ref()
        .expect("a previous step validated input");
    assert_eq!(input.name, name);
}

#[then("the command runs without a terminal")]
fn the_command_runs_without_a_terminal(world: &mut AppWorld) {
    let input = world.input.take().expect("a previous step validated input");
    let output = Example.execute(input).expect("the command succeeds");
    let mut rendered = Vec::new();
    output
        .write_to(&mut rendered)
        .expect("rendering cannot fail");

    assert!(
        String::from_utf8(rendered)
            .expect("the example writes UTF-8")
            .contains("hello, ada"),
        "the command produced the expected data"
    );
}

#[scenario(
    path = "tests/features/facet_bridge.feature",
    name = "An alias resolves to the same typed input"
)]
#[test]
fn an_alias_resolves_to_the_same_typed_input(world: AppWorld) {
    let _ = world;
}

/// A command declared with one attribute per field, as decision-13 describes it.
///
/// Its fields are read by clap's derive, not by hand, which is why the module allows the
/// dead-code lint a struct with no reader of its own would otherwise earn.
///
/// It lives here rather than in the reference application on purpose: CLI-28's first
/// criterion is the grammar half, and the example moves over once the interaction and
/// normalization halves exist. Declaring it in an application-shaped test is the point —
/// this is a consumer of the public macro, not a second implementation of it.
#[allow(
    dead_code,
    reason = "clap's derive is the reader, and the test drives the API"
)]
mod declared {
    use clifacet::clap::CommandFactory as _;

    #[clifacet::command(input = DeclaredInput)]
    #[command(name = "declare", about = "One attribute per field")]
    pub struct Declared {
        /// Who to greet. Defaults to `world`.
        #[clifacet(long, short = 'n', alias = "nickname", value_name = "NAME")]
        pub name: Option<String>,

        /// How many times to greet.
        #[clifacet(long, value_name = "COUNT")]
        pub repeat: u8,

        /// Anything the key table does not cover stays clap's, untouched.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=8))]
        pub level: u8,
    }

    /// The typed input the derive builds field by field.
    pub struct DeclaredInput {
        pub name: Option<String>,
        pub repeat: u8,
        pub level: u8,
    }

    /// The rendered help, which is clap's because the keys were forwarded into `#[arg]`.
    pub fn help() -> String {
        Declared::command().render_help().to_string()
    }
}

#[given("a command whose arguments are declared with one attribute per field")]
fn a_command_declares_its_arguments_once(world: &mut AppWorld) {
    // Placeholder: the given step establishes the declaration, the when step asks.
    // The declaration is the fixture: it either compiles into a clap command or the
    // scenario does not run.
    world.help = declared::help();
}

#[when("the user asks for help")]
fn the_user_asks_for_help(world: &mut AppWorld) {
    world.help = declared::help();
}

#[then("the help shows the short flag, the value name and the field's doc comment")]
fn the_help_is_built_from_the_declaration(world: &AppWorld) {
    assert!(
        world.help.contains("-n, --name <NAME>"),
        "the short flag and value name reached clap:\n{}",
        world.help
    );
    assert!(
        world.help.contains("Who to greet"),
        "the doc comment is the help text:\n{}",
        world.help
    );
}

#[then("the same declaration parses a value through its alias")]
fn the_declaration_parses_through_the_alias(_world: &mut AppWorld) {
    use clifacet::clap::Parser as _;

    let cli = declared::Declared::try_parse_from([
        "declare",
        "--nickname",
        "ada",
        "--repeat",
        "3",
        "--level",
        "2",
    ])
    .expect("the forwarded grammar parses");
    assert_eq!(cli.name.as_deref(), Some("ada"));
    assert_eq!(cli.repeat, 3);
    assert_eq!(cli.level, 2);
}

#[scenario(
    path = "tests/features/facet_bridge.feature",
    name = "A field declared once carries its own grammar"
)]
#[test]
fn a_field_declared_once_carries_its_own_grammar(world: AppWorld) {
    let _ = world;
}
