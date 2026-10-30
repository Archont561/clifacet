//! Properties the command core promises, over inputs no table would enumerate.
//!
//! decision-6 asks for proptest where a real invariant exists. Three do:
//!
//! 1. A validated input survives a round trip through argv, so no field is lost
//!    in the bridge and the flags are a faithful rendering of the input.
//! 2. Resolution is deterministic: the same argv never produces two different
//!    answers, however odd the arguments are.
//! 3. The application never panics and never invents an exit code outside its
//!    documented three, whatever reaches it.
//!
//! Names and repeats outside the accepted range are *not* claimed to round trip.
//! Validation normalizes them (trimming, bounds), so the round trip is stated over
//! canonical input, and normalization itself is a separate property below.

use clifacet::Facet;
use clifacet::clap::Parser as _;
use clifacet::example::{self, Cli, ExampleInput, Level, MAX_REPEAT};
use clifacet::prompt::Scripted;
use proptest::prelude::*;

/// A name validation accepts unchanged: non-empty, already trimmed.
fn canonical_name() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9][a-zA-Z0-9 ,.'-]{0,11}"
        .prop_map(|name| name.trim_end().to_owned())
        .prop_filter("a canonical name is not blank", |name| !name.is_empty())
}

fn canonical_input() -> impl Strategy<Value = ExampleInput> {
    (
        canonical_name(),
        1..=MAX_REPEAT,
        prop_oneof![Just(Level::Basic), Just(Level::Verbose)],
        any::<bool>(),
    )
        .prop_map(|(name, repeat, level, explode)| ExampleInput {
            name,
            repeat,
            level,
            explode,
            token: None,
        })
}

/// The argv that asks for exactly this input.
fn argv_for(input: &ExampleInput) -> Vec<String> {
    let mut argv = vec![
        String::from("example"),
        String::from("--name"),
        input.name.clone(),
        String::from("--repeat"),
        input.repeat.to_string(),
        String::from("--level"),
        input.level.to_string(),
    ];
    if input.explode {
        argv.push(String::from("--explode"));
    }
    argv
}

/// Runs the application entry point with the given arguments, capturing everything.
///
/// The program name is prepended, because that is what a process receives and what
/// the entry point expects to find at index 0. Capabilities are stated rather than
/// detected and the frontend is scripted, so no property can reach a real terminal.
fn run(args: &[String]) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let full = std::iter::once(String::from("example")).chain(args.iter().cloned());
    let code = example::main_with(
        full,
        clifacet::policy::Capabilities::headless(),
        || Box::new(Scripted::default()),
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

proptest! {
    // Seeds live beside the tests under a name proptest documents, so a failure
    // found on one machine re-runs on the next (decision-6: commit useful seeds).
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: Some(Box::new(
            proptest::test_runner::FileFailurePersistence::WithSource("proptest-regressions"),
        )),
        ..ProptestConfig::default()
    })]

    /// The public seam is lossless for input it accepts.
    #[test]
    fn a_canonical_input_round_trips_through_argv(input in canonical_input()) {
        let argv = argv_for(&input);
        let draft = Cli::try_parse_from(&argv)
            .map_err(|error| TestCaseError::fail(format!("{argv:?} should parse: {error}")))?;
        let validated = draft
            .validate()
            .map_err(|failure| TestCaseError::fail(format!("{argv:?} should validate: {failure}")))?;

        prop_assert_eq!(&validated, &input, "argv: {:?}", argv);
    }

    /// Odd arguments are answered, not guessed twice.
    #[test]
    fn resolution_is_deterministic(argv in proptest::collection::vec(argument(), 0..6)) {
        let first = run(&argv);
        let second = run(&argv);

        prop_assert_eq!(first, second, "argv: {:?}", argv);
    }

    /// They are also answered within the contract: three exit codes, no panic.
    #[test]
    fn every_argv_lands_on_a_documented_exit_code(argv in proptest::collection::vec(argument(), 0..6)) {
        let (code, out, err) = run(&argv);

        prop_assert!(
            [
                example::EXIT_SUCCESS,
                example::EXIT_EXECUTION_FAILED,
                example::EXIT_VALIDATION_FAILED,
                example::EXIT_CANCELLED,
            ]
            .contains(&code),
            "argv {:?} produced exit {}",
            argv,
            code
        );
        // Success is data with no diagnostic; every failure explains itself on the
        // diagnostic stream. Neither stream is ever written alongside the other.
        prop_assert!(code != example::EXIT_SUCCESS || err.is_empty(), "argv: {:?}", argv);
        prop_assert!(code == example::EXIT_SUCCESS || !err.is_empty(), "argv: {:?}", argv);
        prop_assert!(out.is_empty() || err.is_empty(), "argv: {:?}", argv);
    }

    /// Validation normalizes rather than trusts: an accepted name is trimmed, and a
    /// blank one is refused instead of silently becoming the default.
    #[test]
    fn a_name_is_trimmed_or_refused(name in ".{0,8}") {
        let argv = vec![
            String::from("example"),
            String::from("--name"),
            name.clone(),
        ];
        let Some(draft) = Cli::try_parse_from(&argv).ok() else {
            return Ok(());
        };

        match draft.validate() {
            Ok(input) => {
                prop_assert_eq!(&input.name, &name.trim(), "name {:?}", name);
                prop_assert!(!input.name.is_empty(), "name {:?}", name);
            }
            Err(failure) => {
                prop_assert_eq!(failure.kind(), clifacet::FailureKind::Validation);
                prop_assert!(name.trim().is_empty(), "only a blank name is refused: {:?}", name);
            }
        }
    }
}

/// One argument a user could plausibly type, plus the ones an attacker would.
fn argument() -> impl Strategy<Value = String> {
    let fixed = prop_oneof![
        Just(String::from("--name")),
        Just(String::from("--nickname")),
        Just(String::from("-n")),
        Just(String::from("--repeat")),
        Just(String::from("--level")),
        Just(String::from("--explode")),
        Just(String::from("--interactive")),
        Just(String::from("--tui")),
        Just(String::from("--yes")),
        Just(String::from("--help")),
        Just(String::from("-h")),
        Just(String::from("--version")),
        Just(String::from("--")),
        Just(String::from("ada")),
        Just(String::from("verbose")),
        Just(String::from("chatty")),
        Just(String::from("0")),
        Just(String::from("8")),
        Just(String::from("9")),
        Just(String::from("255")),
        Just(String::new()),
        Just(String::from("  ")),
    ];
    prop_oneof![fixed, ".{0,6}"]
}
