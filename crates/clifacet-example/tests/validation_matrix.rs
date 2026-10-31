//! The three layers of the example command, as tables instead of prose.
//!
//! decision-6 asks for rstest fixtures and case tables around the agreed seams.
//! The layers are distinct promises, so they get distinct tables:
//!
//! - **draft** — what `clap` collected, before any application rule runs. This is
//!   the state a later frontend fills in, so its shape is a contract.
//! - **input** — the validated value. Normalization happens on the way in, and
//!   every value of this type is runnable.
//! - **validation** — what is refused, with the message the user sees.
//!
//! A fixture chain (`argv` → `draft` → `input`) keeps each case to the one value it
//! is about, so a failure names the layer rather than the plumbing.

use clifacet::clap::Parser as _;
use clifacet::{Command, Facet, Failure, FailureKind};
use clifacet_example::{Cli, Example, ExampleInput, Level, MAX_REPEAT};
use rstest::{fixture, rstest};

/// Parses the given argv into the draft `clap` produced.
///
/// # Panics
///
/// Panics when `clap` rejects the arguments: this fixture is only used by cases
/// that are expected to parse, so a parse error there is a broken case.
#[fixture]
fn draft(#[default(&[])] argv: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("example").chain(argv.iter().copied()))
        .unwrap_or_else(|error| panic!("{argv:?} should parse: {error}"))
}

/// Validates the draft, going through the same bridge the binary uses.
///
/// # Panics
///
/// Panics when validation fails: the cases using this fixture are the ones that
/// must be accepted, and `rejects` covers the rest.
#[fixture]
fn input(#[default(&[])] argv: &[&str]) -> ExampleInput {
    draft(argv)
        .validate()
        .unwrap_or_else(|failure| panic!("{argv:?} should validate: {failure}"))
}

/// The failure the draft produces, for the cases that must be refused.
fn rejects(argv: &[&str]) -> Failure {
    draft(argv)
        .validate()
        .err()
        .unwrap_or_else(|| panic!("{argv:?} should be refused"))
}

/// The shape of the draft, before validation touches it.
#[rstest]
#[case::empty(&[], None, 1, Level::Basic, false)]
#[case::name(&["--name", "ada"], Some("ada"), 1, Level::Basic, false)]
#[case::trimmed_later(&["--name", "  ada  "], Some("  ada  "), 1, Level::Basic, false)]
#[case::alias(&["--nickname", "ada"], Some("ada"), 1, Level::Basic, false)]
#[case::short(&["-n", "ada"], Some("ada"), 1, Level::Basic, false)]
#[case::repeat(&["--repeat", "3"], None, 3, Level::Basic, false)]
#[case::level(&["--level", "verbose"], None, 1, Level::Verbose, false)]
#[case::explode(&["--explode"], None, 1, Level::Basic, true)]
#[case::out_of_range_draft(&["--repeat", "9"], None, 9, Level::Basic, false)]
fn the_draft_is_what_clap_collected(
    #[case] argv: &[&str],
    #[with(argv)] draft: Cli,
    #[case] name: Option<&str>,
    #[case] repeat: u8,
    #[case] level: Level,
    #[case] explode: bool,
) {
    assert_eq!(
        (
            draft.name.as_deref(),
            draft.repeat,
            draft.level,
            draft.explode
        ),
        (name, repeat, level, explode),
        "draft collected from {argv:?}"
    );
}

/// The input, after normalization and the application's own rules.
#[rstest]
#[case::default_name(&[], "world", 1, Level::Basic, false)]
#[case::name_is_trimmed(&["--name", "  ada  "], "ada", 1, Level::Basic, false)]
#[case::alias_reaches_the_same_input(&["--nickname", "ada"], "ada", 1, Level::Basic, false)]
#[case::short_alias_too(&["-n", "ada"], "ada", 1, Level::Basic, false)]
#[case::upper_bound_is_valid(
    &["--repeat", "8"],
    "world",
    MAX_REPEAT,
    Level::Basic,
    false,
)]
#[case::every_flag_together(
    &["--name", "ada", "--repeat", "2", "--level", "verbose", "--explode"],
    "ada",
    2,
    Level::Verbose,
    true,
)]
fn the_input_is_the_validated_form(
    #[case] argv: &[&str],
    #[with(argv)] input: ExampleInput,
    #[case] name: &str,
    #[case] repeat: u8,
    #[case] level: Level,
    #[case] explode: bool,
) {
    assert_eq!(
        (&input.name, input.repeat, input.level, input.explode),
        (&name.to_owned(), repeat, level, explode),
        "input derived from {argv:?}"
    );
}

/// Every validated input is runnable, which is what `Command::execute` asserts by
/// taking it by value.
#[rstest]
#[case::empty(&[])]
#[case::every_flag(&["--name", "ada", "--repeat", "8", "--level", "verbose"])]
#[case::trimmed(&["--name", " ada "])]
fn the_input_runs_without_a_terminal(#[case] argv: &[&str], #[with(argv)] input: ExampleInput) {
    let output = Example
        .execute(input)
        .unwrap_or_else(|failure| panic!("{argv:?} produced runnable input: {failure}"));

    assert!(!output.lines.is_empty(), "{argv:?} produced data");
}

/// What the application refuses, in the words the user reads.
#[rstest]
#[case::zero(&["--repeat", "0"], "--repeat must be between 1 and 8, got 0")]
#[case::above_the_bound(&["--repeat", "9"], "--repeat must be between 1 and 8, got 9")]
#[case::largest_u8(&["--repeat", "255"], "--repeat must be between 1 and 8, got 255")]
#[case::blank_name(&["--name", ""], "--name must not be blank")]
#[case::whitespace_name(&["--name", "   "], "--name must not be blank")]
#[case::blank_alias(&["--nickname", " "], "--name must not be blank")]
fn validation_refuses_and_says_why(#[case] argv: &[&str], #[case] expected: &str) {
    let failure = rejects(argv);

    assert_eq!(failure.kind(), FailureKind::Validation, "{argv:?}");
    assert_eq!(failure.message(), expected, "{argv:?}");
}

/// Parse errors are `clap`'s and never reach validation, so the two failure
/// sources stay distinguishable to the application.
#[rstest]
#[case::unknown_flag(&["--nope"])]
#[case::missing_value(&["--name"])]
#[case::bad_level(&["--level", "chatty"])]
#[case::not_a_number(&["--repeat", "many"])]
fn a_usage_error_never_becomes_a_validation_failure(#[case] argv: &[&str]) {
    let error = Cli::try_parse_from(std::iter::once("example").chain(argv.iter().copied()))
        .expect_err("clap refuses this");

    assert_eq!(error.exit_code(), clifacet_example::EXIT_USAGE, "{argv:?}");
}
