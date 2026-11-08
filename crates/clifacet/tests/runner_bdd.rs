//! The runner's pipeline, as a user of an application sees it (decision-19 item 1,
//! CLI-46).
//!
//! The application is only the scenario: a `clap` command, a planned command that
//! records what it writes, and [`App`] running one against the other. `runner.rs` proves
//! each step and outcome case by case.

use std::cell::RefCell;
use std::rc::Rc;

use clifacet::command::{Plan, Planned};
use clifacet::contract::ExitPolicy;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Prompter, Scripted};
use clifacet::{App, Failure, Globals};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// Save a file, overwriting only after a confirmation.
#[clifacet::command(input = Save)]
#[derive(Debug, Clone)]
#[command(name = "save", about = "Save a file")]
struct SaveCli {
    /// The file to write.
    file: String,

    /// Overwrite the file if it exists.
    #[clifacet(long, confirm = "Overwrite the file?")]
    force: bool,

    #[command(flatten)]
    #[clifacet(skip)]
    globals: Globals,
}

struct Save {
    file: String,
    force: bool,
}

/// The command that runs: it plans one write and records the write when it happens.
struct Writer {
    written: Rc<RefCell<Vec<String>>>,
}

impl Planned for Writer {
    type Input = Save;
    type Output = String;

    fn plan(&self, save: Save) -> Result<Plan<'_, String>, Failure> {
        let step = format!("write {}", save.file);
        let work = move || {
            let _ = save.force;
            self.written.borrow_mut().push(save.file.clone());
            Ok(format!("saved {}", save.file))
        };
        Ok(Plan::new(work).step(step))
    }
}

/// The application, what the terminal can answer, and how the last run ended.
struct World {
    writer: Writer,
    capabilities: Capabilities,
    answers: Vec<Answer>,
    code: Option<i32>,
    out: String,
    err: String,
}

#[fixture]
fn world() -> World {
    World {
        writer: Writer {
            written: Rc::default(),
        },
        capabilities: Capabilities::headless(),
        answers: Vec::new(),
        code: None,
        out: String::new(),
        err: String::new(),
    }
}

#[given("an application whose save command records every file it writes")]
fn an_application(world: &mut World) {
    world.writer.written.borrow_mut().clear();
}

#[given("a terminal where the user declines every confirmation")]
fn a_declining_terminal(world: &mut World) {
    world.capabilities = Capabilities::terminal();
    world.answers = vec![Answer::No; 4];
}

#[when("the user runs {command_line:string}")]
fn the_user_runs(world: &mut World, command_line: String) {
    let argv: Vec<String> = command_line
        .trim_matches('"')
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let answers = Scripted::new(world.answers.clone());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = App::<SaveCli>::new().command(&world.writer).run_with(
        argv,
        world.capabilities,
        move || -> Box<dyn Prompter> { Box::new(answers) },
        &mut out,
        &mut err,
    );
    world.code = Some(code);
    world.out = String::from_utf8(out).expect("utf-8 data");
    world.err = String::from_utf8(err).expect("utf-8 diagnostics");
}

#[then("the help is written on the data stream")]
fn the_help_is_written(world: &World) {
    assert!(world.out.contains("Usage: save"), "data: {:?}", world.out);
    assert_eq!(world.err, "", "help is not a diagnostic");
}

#[then("the data stream reads {text:string}")]
fn the_data_stream_reads(world: &World, text: String) {
    assert_eq!(world.out, format!("{}\n", text.trim_matches('"')));
}

#[then("nothing is written on the data stream")]
fn nothing_on_the_data_stream(world: &World) {
    assert_eq!(world.out, "");
}

#[then("the run exits with the success code")]
fn exits_with_success(world: &World) {
    assert_eq!(
        world.code,
        Some(ExitPolicy::standard().success()),
        "diagnostics: {}",
        world.err
    );
}

#[then("the run exits with the cancellation code")]
fn exits_cancelled(world: &World) {
    assert_eq!(
        world.code,
        Some(ExitPolicy::standard().cancelled()),
        "diagnostics: {}",
        world.err
    );
}

#[then("no file was written")]
fn no_file_was_written(world: &World) {
    assert_eq!(*world.writer.written.borrow(), Vec::<String>::new());
}

#[then("the file written was {file:string}")]
fn the_file_written_was(world: &World, file: String) {
    assert_eq!(*world.writer.written.borrow(), [file.trim_matches('"')]);
}

#[scenario(
    path = "features/runner.feature",
    name = "Asking for help runs nothing"
)]
#[test]
fn asking_for_help_runs_nothing(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/runner.feature",
    name = "A dry run shows what would change and changes nothing"
)]
#[test]
fn a_dry_run_changes_nothing(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/runner.feature",
    name = "A declined question cancels the run"
)]
#[test]
fn a_declined_question_cancels(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/runner.feature",
    name = "A real run writes what the dry run showed"
)]
#[test]
fn a_real_run_writes_what_the_dry_run_showed(world: World) {
    let _ = world;
}
