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
