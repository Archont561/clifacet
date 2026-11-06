//! The secret-handling promise, stated as scenarios.
//!
//! Every scenario drives the published surface: the example's own entry point, the
//! provenance type an application records its inputs with, and the session and
//! frontends an application asks for a secret through.

use std::io::Cursor;

use clifacet::policy::{Capabilities, Mode, Policy, Request};
use clifacet::prompt::{Answer, Scripted, Session, Terminal};
use clifacet::provenance::{Provenance, Source};
use clifacet::secret::Secret;
use clifacet::{Failure, FailureKind};
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
    session: Option<Session>,
    secret: Option<Secret<String>>,
    refusal: Option<Failure>,
    shown: Vec<u8>,
    read: u64,
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

#[given("a session that may prompt for a token")]
fn a_session_that_may_prompt(world: &mut World) {
    let plan = Policy::up_to(Mode::Interactive)
        .resolve(Request::none(), Capabilities::terminal())
        .expect("prompts are permitted here");
    world.session = Some(Session::start(plan, || {
        Box::new(Scripted::new([Answer::Text(CANARY.to_owned())]))
    }));
}

#[when("the user types the token where it is asked for")]
fn the_user_types_the_token(world: &mut World) {
    let session = world.session.as_mut().expect("a session was started");
    world.secret = session
        .ask_secret(None, "token", "API token?")
        .expect("the token was typed");
}

#[then("the token is reachable only through Secret")]
fn reachable_only_through_secret(world: &World) {
    let secret = world.secret.as_ref().expect("the token was asked for");
    assert!(secret.expose() == CANARY, "expose() is the one read");
    let session = world.session.as_ref().expect("a session was started");
    for shown in [
        secret.to_string(),
        format!("{secret:?}"),
        format!("{session:?}"),
        session.provenance().to_string(),
    ] {
        assert!(!shown.contains(CANARY), "the token leaked into a rendering");
    }
}

#[then("provenance records it as a secret from the prompt")]
fn recorded_as_a_prompted_secret(world: &World) {
    let provenance = world
        .session
        .as_ref()
        .expect("a session was started")
        .provenance();
    assert_eq!(provenance.source_of("token"), Some(Source::Prompt));
    assert!(provenance.is_secret("token"));
}

#[given("a line-reading terminal")]
fn a_line_reading_terminal(_world: &mut World) {}

#[when("a token is asked for through it")]
fn a_token_is_asked_for_through_it(world: &mut World) {
    let mut input = Cursor::new(format!("{CANARY}\n"));
    let mut shown = Vec::new();
    let refusal = {
        let mut terminal = Terminal::new(&mut input, &mut shown);
        clifacet::prompt::Prompter::ask_secret(&mut terminal, "API token?").err()
    };
    world.refusal = refusal;
    world.shown = shown;
    world.read = input.position();
}

#[then("it refuses before the question is shown or anything is read")]
fn it_refuses_before_asking(world: &World) {
    let refusal = world.refusal.as_ref().expect("the terminal refused");
    assert_eq!(refusal.kind(), FailureKind::Execution);
    assert!(refusal.message().contains("echo"), "{}", refusal.message());
    assert!(world.shown.is_empty(), "the question was never shown");
    assert_eq!(world.read, 0, "not a byte of the answer was read");
}

#[scenario(
    path = "tests/features/secret.feature",
    name = "A prompted secret is recorded, never shown"
)]
#[test]
fn a_prompted_secret_is_recorded_never_shown(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/secret.feature",
    name = "A frontend that cannot hide input does not ask for a secret"
)]
#[test]
fn a_frontend_that_cannot_hide_input_refuses(world: World) {
    let _ = world;
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
