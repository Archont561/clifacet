//! The program the wrapper spawns is the application's context, not an environment
//! variable (CLI-48, AC#2).
//!
//! `CLIFACET_BACKLOG_BIN` made the wrapper look up its dependency at runtime, so every test
//! that wanted a fake had to mutate the process environment — which is shared, and which a
//! test running in parallel would see. Now the program arrives the way every other
//! dependency does: the application builds it, and a test injects a different one.

use std::convert::Infallible;

use clifacet::policy::Capabilities;
use clifacet::prompt::Scripted;
use clifacet_backlog_lite::{Program, Request, app, invocation_of, run_app};

/// The wrapper's context: the program it forwards to.
fn fake() -> Program {
    Program::new(["bl-fake"])
}

/// Runs `bl` with the fake program in its context, and answers with (code, out, err).
fn run(arguments: &[&str]) -> (i32, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = app(|| Ok::<_, Infallible>(fake())).run_with(
        std::iter::once("bl").chain(arguments.iter().copied()),
        Capabilities::headless(),
        || Box::new(Scripted::default()),
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8(out).expect("UTF-8"),
        String::from_utf8(err).expect("UTF-8"),
    )
}

#[test]
fn the_program_comes_from_the_context() {
    let request = Request {
        json: false,
        command: None,
    };

    assert_eq!(
        invocation_of(request, &fake()).argv()[0],
        "bl-fake",
        "the context decides what runs"
    );
}

#[test]
fn the_environment_variable_is_gone() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/invocation.rs"))
        .expect("the invocation module is readable");
    let program = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/program.rs"))
        .expect("the program module is readable");

    for source in [source, program] {
        assert!(
            !source.contains("CLIFACET_BACKLOG_BIN"),
            "the override is the context now, not a variable a test has to set"
        );
        assert!(
            !source.contains("env::var"),
            "nothing about what runs is read from the process environment"
        );
    }
}

#[test]
fn the_binary_names_the_real_program_and_a_test_names_its_own() {
    // The binary's context is `Program::default`, and a test's is what it passes; nothing
    // about either is read back from the environment.
    assert_eq!(Program::default().argv(), ["backlog"]);
    assert_eq!(fake().argv(), ["bl-fake"]);
    let _ = run_app;
}

#[test]
fn a_dry_run_names_the_program_the_context_gave() {
    let (code, out, err) = run(&["--dry-run", "show", "CLI-1"]);

    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("bl-fake task view CLI-1 --plain"),
        "the dry run names the injected program: {out}"
    );
}

#[test]
fn an_invocation_still_describes_itself_without_a_process() {
    let invocation = invocation_of(
        Request {
            json: true,
            command: None,
        },
        &fake(),
    );

    assert_eq!(invocation.shell_line(), "bl-fake task list --json");
    assert!(invocation.explain().contains("list"));
}
