//! The dry run is the plan the real run executes, as an executable scenario.
//!
//! The real run spawns the wrapped tool, so this scenario stops at the preview and
//! compares it with the plan `Backlog` builds for the same command line; that executing
//! a plan is the only way a planned command runs is the library's, and is proven in
//! `crates/clifacet/tests/plan.rs`.

use clap::Parser as _;
use clifacet::Facet;
use clifacet::command::Planned as _;
use clifacet::policy::Capabilities;
use clifacet::prompt::Scripted;
use clifacet_backlog_lite::{Backlog, Cli, EXIT, app, request_of};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// One previewed run of `bl`, and the command line it was given.
struct World {
    capabilities: Capabilities,
    arguments: Vec<String>,
    code: i32,
    out: String,
    err: String,
    asked: Vec<String>,
}

#[fixture]
fn world() -> World {
    World {
        capabilities: Capabilities::headless(),
        arguments: Vec::new(),
        code: -1,
        out: String::new(),
        err: String::new(),
        asked: Vec::new(),
    }
}

fn parsed(arguments: &[String], dry_run: bool) -> Cli {
    let flag = dry_run.then_some("--dry-run");
    Cli::try_parse_from(
        ["bl"]
            .into_iter()
            .chain(flag)
            .chain(arguments.iter().map(String::as_str)),
    )
    .expect("bl parses its own command line")
}

#[given("bl runs without a terminal")]
fn without_a_terminal(world: &mut World) {
    world.capabilities = Capabilities::headless();
}

#[when("the user previews creating a task titled {title} with one criterion")]
fn previews_a_new_task(world: &mut World, title: String) {
    world.arguments = ["new", title.trim_matches('"'), "--ac", "it ships"]
        .map(String::from)
        .to_vec();
    let script = Scripted::new([]);
    let probe = script.clone();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    world.code = app(
        parsed(&world.arguments, true),
        world.capabilities,
        move || Box::new(probe),
        &mut out,
        &mut err,
    );
    world.out = String::from_utf8(out).expect("UTF-8");
    world.err = String::from_utf8(err).expect("UTF-8");
    world.asked = script.asked();
}

#[then("the output is exactly the plan bl would execute")]
fn the_output_is_the_plan(world: &World) {
    let request = request_of(Facet::validate(parsed(&world.arguments, false)).expect("valid"));
    let plan = Backlog.plan(request).expect("the request plans");

    assert_eq!(world.out, format!("{}\n", plan.describe()));
    assert!(
        plan.describe()
            .contains("task create 'Ship it' --ac 'it ships'"),
        "the runnable line is part of the plan: {}",
        plan.describe()
    );
}

#[then("the run succeeded without asking anything")]
fn succeeded_without_asking(world: &World) {
    assert_eq!(world.code, EXIT.success(), "{}", world.err);
    assert!(world.err.is_empty(), "{}", world.err);
    assert_eq!(world.asked, [""; 0].map(String::from));
}

#[scenario(
    path = "features/plan.feature",
    name = "A dry run prints the plan, and only the plan"
)]
#[test]
fn a_dry_run_prints_the_plan(world: World) {
    let _ = world;
}
