//! The secret-handling promise, stated as scenarios.
//!
//! Both scenarios drive the published surface: the example's own entry point, and the
//! provenance type an application records its inputs with.

use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Scripted};
use clifacet::provenance::{Provenance, Source};
use clifacet_example as example;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// A value that must not survive into any framework-owned output.
const CANARY: &str = "sk-canary-bdd-1c4e";

/// What one scenario has collected.
#[derive(Default)]
struct World {
    token: String,
    code: i32,
    data: String,
    diagnostic: String,
    provenance: Provenance,
}

#[fixture]
fn world() -> World {
    World::default()
}

#[given("a credential the example refuses")]
fn a_refused_credential(world: &mut World) {
    world.token = format!("revoked-{CANARY}");
}

#[when("the example is run with it")]
fn the_example_runs_with_it(world: &mut World) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    world.code = example::main_with(
        [
            String::from("example"),
            String::from("--name"),
            String::from("ada"),
            String::from("--token"),
            world.token.clone(),
        ],
        Capabilities::headless(),
        || Box::new(Scripted::default()),
        &mut out,
        &mut err,
    );
    world.data = String::from_utf8_lossy(&out).into_owned();
    world.diagnostic = String::from_utf8_lossy(&err).into_owned();
}

#[then("the diagnostic explains the rejection")]
fn the_diagnostic_explains(world: &World) {
    assert_eq!(world.code, example::EXIT_EXECUTION_FAILED);
    assert_eq!(world.data, "", "nothing is reported as data");
    assert!(
        world.diagnostic.contains("was rejected"),
        "the diagnostic says what happened"
    );
    assert!(
        world.diagnostic.contains("[redacted]"),
        "the credential is replaced by the marker"
    );
}

#[then("the credential value appears nowhere")]
fn the_value_is_nowhere(world: &World) {
    // No `{:?}` of the collected strings in these messages: a test that prints the
    // canary to report its own failure has already leaked it.
    assert!(
        !world.diagnostic.contains(CANARY),
        "the diagnostic leaks the value"
    );
    assert!(
        !world.data.contains(CANARY),
        "the data stream leaks the value"
    );
}

#[given("a run whose answer came from a prompt")]
fn an_answer_from_a_prompt(world: &mut World) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::main_with(
        [String::from("example")],
        Capabilities::terminal(),
        || Box::new(Scripted::new([Answer::Text(String::from("ada"))])),
        &mut out,
        &mut err,
    );

    assert_eq!(code, example::EXIT_SUCCESS, "the prompted run succeeds");
    world.provenance.record("name", Source::Prompt);
}

#[when("the run's provenance is recorded")]
fn the_provenance_is_recorded(world: &mut World) {
    world.provenance.record_secret("token", Source::Environment);
}

#[then("it names the answer and its source")]
fn it_names_the_source(world: &World) {
    assert_eq!(world.provenance.source_of("name"), Some(Source::Prompt));
    assert_eq!(
        world.provenance.source_of("token"),
        Some(Source::Environment)
    );
    assert!(
        world.provenance.to_string().contains("name <- prompt"),
        "the record is readable"
    );
}

#[then("it holds no value at all")]
fn it_holds_no_value(world: &World) {
    let shown = world.provenance.to_string();

    assert!(shown.contains("(secret)"), "a secret is marked as one");
    assert!(!shown.contains("ada"), "an answer is not recorded either");
    assert!(!shown.contains(CANARY), "no value is recorded");
}

#[scenario(
    path = "tests/features/secret.feature",
    name = "A rejected credential is redacted"
)]
#[test]
fn a_rejected_credential_is_redacted(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/secret.feature",
    name = "Provenance names the source of an answer"
)]
#[test]
fn provenance_names_the_source(world: World) {
    let _ = world;
}
