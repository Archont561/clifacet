//! What discovery promises, asserted through the public API of a real application (CLI-50).
//!
//! This crate exists to be built: `build.rs` scans `src/commands/`, and `mod.rs` registers
//! what it found with no list. Everything here drives the result — the parsed command line
//! and the tree it builds — because that is what a caller sees.

use clap::{CommandFactory, Parser as _};
use clifacet::{Facet, FailureKind};

use clifacet_discovered::{Cli, Request};

/// How a test reaches the root command, through the trait that builds it.
fn command_of() -> clap::Command {
    <Cli as CommandFactory>::command()
}

/// The subcommand names the root offers, in clap's own order.
fn subcommands() -> Vec<String> {
    command_of()
        .get_subcommands()
        .filter(|command| command.get_name() != "help")
        .map(|command| command.get_name().to_owned())
        .collect()
}

#[test]
fn the_folder_registers_itself_with_no_list_anywhere() {
    let mut found = subcommands();
    found.sort();

    assert_eq!(
        found,
        ["db", "echo", "list"],
        "`list.rs` and `echo.rs` are commands and `db/` is a group, all found by the build script"
    );
}

#[test]
fn a_group_is_a_folder_that_registered_itself_too() {
    let root = command_of();
    let db = root
        .find_subcommand("db")
        .expect("the group is a subcommand");

    let nested: Vec<String> = db
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect();

    assert_eq!(
        nested,
        ["migrate"],
        "the group's own `commands!()` was discovered from `db/`"
    );
}

#[test]
fn a_helper_is_declared_but_never_a_command() {
    let found = subcommands();

    assert!(
        !found.iter().any(|name| name.starts_with('_')),
        "a `_name.rs` helper is not registered: {found:?}"
    );
    // It is still reachable from the folder's commands, which is the point of a helper.
    assert!(
        clifacet_discovered::commands::greeting().contains("hello"),
        "the folder's own commands use the helper"
    );
}

#[test]
fn a_discovered_leaf_validates_like_a_written_one() {
    let cli = Cli::try_parse_from(["discovered", "echo", "  hi  "]).expect("parses");

    let input = Facet::validate(cli).expect("validates");

    let Some(Request::Echo(echoed)) = input.command else {
        panic!("the echo command was chosen");
    };
    assert_eq!(
        echoed.words, "hi",
        "the leaf's own `#[clifacet(trim)]` still applies"
    );
}

#[test]
fn a_discovered_leaf_rejects_what_its_own_rules_reject() {
    let cli = Cli::try_parse_from(["discovered", "echo", "   "]).expect("parses");

    let failure = Facet::validate(cli).expect_err("a blank word is refused");

    assert_eq!(failure.kind(), FailureKind::Validation);
}

#[test]
fn the_root_is_named_by_the_call_not_by_cargo() {
    let root = command_of();

    assert_eq!(root.get_name(), "discovered");
    assert_eq!(
        root.get_about().map(ToString::to_string).as_deref(),
        Some("A proof that a folder registers itself")
    );
}
