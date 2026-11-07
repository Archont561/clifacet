//! A planned command's preview and its run, as an application sees them (decision-18
//! item 5, CLI-31).
//!
//! The application here is only the scenario: it implements `Planned`, previews through
//! `Plan::steps`, and runs through the `Command` impl the library provides for every
//! planned command. `plan.rs` proves the mechanics case by case.

use std::cell::RefCell;
use std::rc::Rc;

use clifacet::command::{Plan, Planned};
use clifacet::{Command, Failure, FailureKind};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// Deploys each named service, or refuses an empty list before doing anything.
struct Deploy {
    /// What was actually deployed, in order.
    deployed: Rc<RefCell<Vec<String>>>,
}

impl Planned for Deploy {
    type Input = Vec<String>;
    type Output = usize;

    fn plan(&self, services: Vec<String>) -> Result<Plan<'_, usize>, Failure> {
        if services.is_empty() {
            return Err(Failure::validation("name at least one service"));
        }
        let steps: Vec<String> = services
            .iter()
            .map(|service| format!("deploy {service}"))
            .collect();
        let deployed = Rc::clone(&self.deployed);
        let plan = Plan::new(move || {
            for service in &services {
                deployed.borrow_mut().push(format!("deploy {service}"));
            }
            Ok(services.len())
        });
        Ok(steps.into_iter().fold(plan, Plan::step))
    }
}

/// The command, what its preview listed, and how its run ended.
struct World {
    deploy: Option<Deploy>,
    preview: Vec<String>,
    run: Option<Result<usize, Failure>>,
}

impl World {
    fn deploy(&self) -> &Deploy {
        self.deploy.as_ref().expect("a deploy command was given")
    }

    fn deployed(&self) -> Vec<String> {
        self.deploy().deployed.borrow().clone()
    }
}

#[fixture]
fn world() -> World {
    World {
        deploy: None,
        preview: Vec::new(),
        run: None,
    }
}

fn services(names: &[&str]) -> Vec<String> {
    names
        .iter()
        .map(|name| name.trim_matches('"').to_owned())
        .collect()
}

#[given("a deploy command that plans before it runs")]
fn a_deploy_command(world: &mut World) {
    world.deploy = Some(Deploy {
        deployed: Rc::default(),
    });
}

#[when("the application previews deploying {first:string} and {second:string}")]
fn previews(world: &mut World, first: String, second: String) {
    let preview = world
        .deploy()
        .plan(services(&[&first, &second]))
        .expect("a valid request plans")
        .steps()
        .to_vec();
    world.preview = preview;
}

#[when("the application runs deploying {first:string} and {second:string}")]
fn runs(world: &mut World, first: String, second: String) {
    world.run = Some(world.deploy().execute(services(&[&first, &second])));
}

#[when("the application runs deploying nothing")]
fn runs_nothing(world: &mut World) {
    world.run = Some(world.deploy().execute(Vec::new()));
}

#[then("the preview lists {first:string} then {second:string}")]
fn the_preview_lists(world: &World, first: String, second: String) {
    assert_eq!(world.preview, [first, second]);
}

#[then("nothing was deployed")]
fn nothing_was_deployed(world: &World) {
    assert!(
        world.deployed().is_empty(),
        "deployed: {:?}",
        world.deployed()
    );
}

#[then("what was deployed is what the preview listed")]
fn deployed_what_was_previewed(world: &World) {
    assert!(!world.preview.is_empty(), "a preview came first");
    assert_eq!(world.deployed(), world.preview);
}

#[then("the run answers {count:u32}")]
fn the_run_answers(world: &World, count: u32) {
    match &world.run {
        Some(Ok(answer)) => assert_eq!(u32::try_from(*answer).ok(), Some(count)),
        other => panic!("the run did not succeed: {other:?}"),
    }
}

#[then("the run fails as invalid input")]
fn fails_as_invalid_input(world: &World) {
    match &world.run {
        Some(Err(failure)) => assert_eq!(failure.kind(), FailureKind::Validation),
        other => panic!("the run did not fail: {other:?}"),
    }
}

#[scenario(
    path = "tests/features/plan.feature",
    name = "A preview lists every step and changes nothing"
)]
#[test]
fn a_preview_changes_nothing(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/plan.feature",
    name = "A real run does exactly what the preview listed"
)]
#[test]
fn a_run_does_what_the_preview_listed(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/plan.feature",
    name = "A request refused while planning runs nothing"
)]
#[test]
fn a_refused_request_runs_nothing(world: World) {
    let _ = world;
}
