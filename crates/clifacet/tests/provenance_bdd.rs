//! Where each value came from, recorded by the library rather than by the application
//! (decision-18 item 4).
//!
//! Every scenario runs a declared command the way an application does: clap parses the
//! line, `Provenance::from_matches` reads where clap found each value, the declared
//! questions are asked on a scripted session, and the session's own record is merged in.
//! `provenance.rs` proves the cases one by one; `secret.feature` in the example covers
//! how a secret is reported.

use clap::{CommandFactory, FromArgMatches as _};
use clifacet::policy::{Capabilities, Mode, Policy, Request as ModeRequest};
use clifacet::prompt::{Answer, Collect, Scripted, Session};
use clifacet::provenance::{Provenance, Source};
use clifacet::{Facet, Failure};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// Every way a value can arrive: typed, asked for, defaulted, or from the environment.
#[clifacet::command(input = DeployInput)]
#[command(name = "deploy")]
struct Deploy {
    /// Who is deploying; asked for when missing.
    #[clifacet(long, ask = "Who is deploying?")]
    name: Option<String>,

    /// How many times.
    #[clifacet(long, default_value_t = 1)]
    repeat: u8,

    /// Read from the environment when not typed. `PATH` is set on every platform the
    /// tests run on, so no scenario has to mutate the process environment.
    #[clifacet(long, env = "PATH")]
    path: Option<String>,
}

#[derive(Debug)]
struct DeployInput {
    #[allow(dead_code, reason = "the scenarios read the record, not the input")]
    name: Option<String>,
    #[allow(dead_code, reason = "the scenarios read the record, not the input")]
    repeat: u8,
    #[allow(dead_code, reason = "the scenarios read the record, not the input")]
    path: Option<String>,
}

/// One run of `deploy`: where its values came from, and what it asked.
struct World {
    capabilities: Capabilities,
    provenance: Provenance,
    asked: Vec<String>,
}

impl World {
    fn run(&mut self, arguments: &[&str], answers: Vec<Answer>) {
        let command = <Deploy as CommandFactory>::command();
        let matches = command
            .clone()
            .try_get_matches_from(std::iter::once("deploy").chain(arguments.iter().copied()))
            .expect("a valid invocation");
        let deploy = Deploy::from_arg_matches(&matches).expect("the parse converts");
        let mut provenance = Provenance::from_matches(&command, &matches);

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
        let input: Result<DeployInput, Failure> =
            deploy.collect(&mut session).and_then(Facet::validate);
        input.expect("the run validates");

        provenance.merge(session.provenance());
        self.provenance = provenance;
        self.asked = script.asked();
    }
}

#[fixture]
fn world() -> World {
    World {
        capabilities: Capabilities::headless(),
        provenance: Provenance::default(),
        asked: Vec::new(),
    }
}

#[given("the run has no terminal")]
fn no_terminal(world: &mut World) {
    world.capabilities = Capabilities::headless();
}

#[given("the run is at a terminal")]
fn at_a_terminal(world: &mut World) {
    world.capabilities = Capabilities::terminal();
}

#[when("the user types {line:string}")]
fn the_user_types(world: &mut World, line: String) {
    let arguments: Vec<&str> = line.split_whitespace().collect();
    world.run(&arguments, Vec::new());
}

#[when("the user types nothing")]
fn the_user_types_nothing(world: &mut World) {
    world.run(&[], Vec::new());
}

#[when("the user types nothing and answers {answer:string}")]
fn the_user_answers(world: &mut World, answer: String) {
    world.run(&[], vec![Answer::Text(answer)]);
}

#[then("{field:string} came from {source:string}")]
fn came_from(world: &World, field: String, source: String) {
    assert_eq!(
        world.provenance.source_of(&field).map(Source::name),
        Some(source.as_str()),
        "{}",
        world.provenance
    );
}

#[then("{field:string} has no source")]
fn has_no_source(world: &World, field: String) {
    assert_eq!(
        world.provenance.source_of(&field),
        None,
        "{}",
        world.provenance
    );
}

#[then("nothing was asked")]
fn nothing_was_asked(world: &World) {
    assert!(world.asked.is_empty(), "asked: {:?}", world.asked);
}

#[then("the record does not contain {value:string}")]
fn the_record_holds_no_value(world: &World, value: String) {
    let shown = format!("{} {:?}", world.provenance, world.provenance);
    assert!(!shown.contains(&value), "{shown}");
}

#[scenario(
    path = "features/provenance.feature",
    name = "Typed, defaulted and environment values are told apart"
)]
#[test]
fn typed_defaulted_and_environment_values(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/provenance.feature",
    name = "A value nobody supplied has no source"
)]
#[test]
fn an_unsupplied_value_has_no_source(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/provenance.feature",
    name = "An answer to a question is recorded as asked, not typed"
)]
#[test]
fn an_answer_is_recorded_as_asked(world: World) {
    let _ = world;
}
