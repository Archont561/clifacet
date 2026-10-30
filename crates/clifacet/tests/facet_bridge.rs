//! The companion derive's slice: `#[derive(Facet)]` wiring a clap command to
//! typed input.
//!
//! These tests drive the public seam only — `clap` parses, `Facet::validate`
//! bridges, and the generated `command()` is clap's own. Nothing here reaches a
//! terminal, and nothing here touches a private clap API.

use clifacet::Facet;
use clifacet::clap::Parser as _;
use clifacet::example::{Cli, ExampleInput, Level};

/// Parses and validates one command line, failing the test on a parse error.
fn validated(args: &[&str]) -> ExampleInput {
    let cli = Cli::try_parse_from(std::iter::once("example").chain(args.iter().copied()))
        .unwrap_or_else(|error| panic!("{args:?} should parse: {error}"));

    // The derived bridge is the only path from clap's parsed struct to input.
    cli.validate()
        .unwrap_or_else(|failure| panic!("{args:?} should validate: {failure}"))
}

#[test]
fn an_alias_reaches_the_same_typed_input_as_its_primary_flag() {
    let long = validated(&["--name", "ada"]);
    let alias = validated(&["--nickname", "ada"]);
    let short = validated(&["-n", "ada"]);

    assert_eq!(long.name, "ada");
    assert_eq!(alias, long, "an alias must not need its own code path");
    assert_eq!(short, long);
}

#[test]
fn the_generated_command_is_clap_owned_grammar() {
    // Same value, not a copy: the derive reports clap's command, so aliases,
    // help and value parsers cannot drift into a second grammar.
    assert_eq!(
        <Cli as Facet>::command().render_long_help().to_string(),
        <Cli as clap::CommandFactory>::command()
            .render_long_help()
            .to_string(),
    );

    let command = <Cli as Facet>::command();
    let name = command
        .get_arguments()
        .find(|argument| argument.get_id() == "name")
        .expect("the name argument exists");
    let aliases: Vec<&str> = name.get_all_aliases().unwrap_or_default();
    assert!(aliases.contains(&"nickname"), "aliases: {aliases:?}");
}

#[test]
fn a_declared_default_survives_the_bridge() {
    let input = validated(&[]);

    assert_eq!(
        input.repeat, 1,
        "the clap default value is the only default"
    );
    assert_eq!(input.level, Level::Basic);
}

#[test]
fn a_custom_value_parser_decides_what_is_valid_before_validation() {
    let input = validated(&["--level", "verbose"]);

    assert_eq!(input.level, Level::Verbose);

    let error = Cli::try_parse_from(["example", "--level", "chatty"])
        .expect_err("the custom parser rejects an unknown level");
    assert!(
        error.to_string().contains("chatty") || error.to_string().contains("level"),
        "the parser's own message must reach the user: {error}"
    );
}

#[test]
fn the_validation_hook_still_rejects_what_the_parser_cannot() {
    let cli = Cli::try_parse_from(["example", "--repeat", "9"]).expect("9 parses as a u8");

    let failure = cli
        .validate()
        .expect_err("the range check is the library's, not the parser's");

    assert_eq!(failure.kind(), clifacet::FailureKind::Validation);
    assert!(failure.message().contains("between 1 and 8"));
}

#[test]
fn the_bridge_carries_every_field_it_declares() {
    let input = validated(&[
        "--name",
        "ada",
        "--repeat",
        "2",
        "--explode",
        "--level",
        "verbose",
    ]);

    assert_eq!(
        input,
        ExampleInput {
            name: String::from("ada"),
            repeat: 2,
            explode: true,
            level: Level::Verbose,
        },
        "a field the derive forgets would silently fall back to its default"
    );
}
