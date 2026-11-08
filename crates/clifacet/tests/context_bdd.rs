//! The application's context, as an author of an application sees it (decision-19,
//! CLI-47).
//!
//! The application is only the scenario: a `collect` that offers what the store already
//! holds, a planned command that runs with the store, and the runner that builds it.
//! `context.rs` proves each step and outcome case by case.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use clifacet::command::{Plan, PlannedWith};
use clifacet::contract::ExitPolicy;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Confirmation, Prompter, Scripted, Session};
use clifacet::{App, Failure, Globals};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// Mark a task done, asking which one if the user did not name it.
#[clifacet::command(input = DoneInput, collect = pick_task, check = one_word)]
#[derive(Debug, Clone)]
#[command(name = "done", about = "Mark a task done")]
struct Done {
    /// The task to mark done.
    #[arg(long)]
    task: Option<String>,

    #[command(flatten)]
    #[clifacet(skip)]
    globals: Globals,
}

struct DoneInput {
    task: Option<String>,
}

/// Validation: a task name is one word, and it runs before the store is built.
fn one_word(input: &DoneInput) -> Result<(), Failure> {
    match &input.task {
        Some(task) if task.contains(' ') => Err(Failure::validation("a task name is one word")),
        _ => Ok(()),
    }
}

/// Asks only when the user did not name the task, and offers what the store holds.
fn pick_task(done: Done, session: &mut Session) -> Result<Done, Failure> {
    let mut done = done;
    if done.task.is_none() && session.plan().asks_the_user() {
        let store = session.context::<Store>()?;
        let open = store.open.borrow().clone();
        done.task = session.choose(None, "Which task?", open)?;
    }
    if let Some(task) = &done.task
        && session.confirm(&format!("Mark {task} done?"))? == Confirmation::Declined
    {
        return Err(Failure::cancelled(format!("{task} stays open")));
    }
    Ok(done)
}

/// The store: what the command depends on, and what it plans with.
struct Store {
    open: RefCell<Vec<String>>,
    done: Rc<RefCell<Vec<String>>>,
}

#[clifacet::planned]
impl PlannedWith<Store> for Done {
    type Input = DoneInput;
    type Output = String;

    fn plan<'a>(&'a self, input: DoneInput, store: &'a Store) -> Result<Plan<'a, String>, Failure> {
        let Some(task) = input.task else {
            return Err(Failure::validation("name a task with --task"));
        };
        if !store.open.borrow().contains(&task) {
            return Err(Failure::validation(format!(
                "there is no open task named {task}"
            )));
        }
        let step = format!("mark {task} done");
        Ok(Plan::new(move || {
            store.done.borrow_mut().push(task.clone());
            Ok(format!("{task} is done"))
        })
        .step(step))
    }
}

/// The application, what the terminal can answer, and how the last run ended.
struct World {
    done: Rc<RefCell<Vec<String>>>,
    store: Result<(), &'static str>,
    capabilities: Capabilities,
    answers: Vec<Answer>,
    code: Option<i32>,
    out: String,
    err: String,
    built: u32,
    asked: Vec<String>,
}

#[fixture]
fn world() -> World {
    World {
        done: Rc::default(),
        store: Ok(()),
        capabilities: Capabilities::headless(),
        answers: Vec::new(),
        code: None,
        out: String::new(),
        err: String::new(),
        built: 0,
        asked: Vec::new(),
    }
}

#[given("an application whose done command marks a task in a store")]
fn an_application(world: &mut World) {
    world.done.borrow_mut().clear();
}

#[given("a terminal where the user declines every confirmation")]
fn a_declining_terminal(world: &mut World) {
    world.capabilities = Capabilities::terminal();
    world.answers = vec![Answer::No; 4];
}

#[given("a terminal where the user picks {task:string}")]
fn a_picking_terminal(world: &mut World, task: String) {
    world.capabilities = Capabilities::terminal();
    world.answers = vec![Answer::Text(task.trim_matches('"').to_owned()), Answer::Yes];
}

#[given("a store that cannot be opened")]
fn a_locked_store(world: &mut World) {
    world.store = Err("the store is locked");
}

#[when("the user runs {command_line:string}")]
fn the_user_runs(world: &mut World, command_line: String) {
    let argv: Vec<String> = command_line
        .trim_matches('"')
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let script = Scripted::new(world.answers.clone());
    let asked = script.clone();
    let built = Rc::new(Cell::new(0));
    let (counter, done) = (Rc::clone(&built), Rc::clone(&world.done));
    let store = world.store;
    let app = App::<Done>::new().context(move || {
        counter.set(counter.get() + 1);
        store.map(|()| Store {
            open: RefCell::new(vec![String::from("a"), String::from("b")]),
            done: Rc::clone(&done),
        })
    });
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = app.run_with(
        argv,
        world.capabilities,
        move || -> Box<dyn Prompter> { Box::new(script) },
        &mut out,
        &mut err,
    );
    world.code = Some(code);
    world.out = String::from_utf8(out).expect("utf-8 data");
    world.err = String::from_utf8(err).expect("utf-8 diagnostics");
    world.built = built.get();
    world.asked = asked.asked();
}

#[then("the help is written on the data stream")]
fn the_help_is_written(world: &World) {
    assert!(world.out.contains("Usage: done"), "data: {:?}", world.out);
    assert_eq!(world.err, "", "help is not a diagnostic");
}

#[then("the data stream reads {text:string}")]
fn the_data_stream_reads(world: &World, text: String) {
    assert_eq!(
        world.out,
        format!("{}\n", text.trim_matches('"')),
        "diagnostics: {}",
        world.err
    );
}

#[then("the diagnostics read {text:string}")]
fn the_diagnostics_read(world: &World, text: String) {
    let text = text.trim_matches('"');
    assert!(
        world.err.contains(text),
        "diagnostics: {:?} did not mention {text:?}",
        world.err
    );
}

#[then("the question asked was {question:string}")]
fn the_question_asked_was(world: &World, question: String) {
    assert_eq!(
        world.asked.first().map(String::as_str),
        Some(question.trim_matches('"')),
        "questions: {:?}",
        world.asked
    );
}

#[then("the context was never built")]
fn the_context_was_never_built(world: &World) {
    assert_eq!(
        world.built, 0,
        "the context was built {} times",
        world.built
    );
}

#[then("the context was built once")]
fn the_context_was_built_once(world: &World) {
    assert_eq!(
        world.built, 1,
        "the context was built {} times",
        world.built
    );
}

#[then("the run marked {task:string} done")]
fn the_run_marked_done(world: &World, task: String) {
    assert_eq!(
        *world.done.borrow(),
        [task.trim_matches('"').to_owned()],
        "diagnostics: {}",
        world.err
    );
}

#[then("no task was marked done")]
fn no_task_was_marked_done(world: &World) {
    assert_eq!(*world.done.borrow(), Vec::<String>::new());
}

#[then("the run exits with the success code")]
fn exits_with_success(world: &World) {
    let policy = ExitPolicy::standard();
    assert_eq!(
        world.code,
        Some(policy.success()),
        "diagnostics: {}",
        world.err
    );
}

#[then("the run exits with the usage code")]
fn exits_with_usage(world: &World) {
    let policy = ExitPolicy::standard();
    assert_eq!(
        world.code,
        Some(policy.validation()),
        "diagnostics: {}",
        world.err
    );
}

#[then("the run exits with the cancellation code")]
fn exits_cancelled(world: &World) {
    let policy = ExitPolicy::standard();
    assert_eq!(
        world.code,
        Some(policy.cancelled()),
        "diagnostics: {}",
        world.err
    );
}

#[then("the run exits with the failure code")]
fn exits_with_failure(world: &World) {
    let policy = ExitPolicy::standard();
    assert_eq!(
        world.code,
        Some(policy.execution()),
        "diagnostics: {}",
        world.err
    );
}

#[scenario(
    path = "features/context.feature",
    name = "A real run marks the task the user named"
)]
#[test]
fn a_real_run_marks_the_task(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/context.feature",
    name = "A dry run previews with the context and changes nothing"
)]
#[test]
fn a_dry_run_previews_with_the_context(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/context.feature",
    name = "Asking for help builds no context"
)]
#[test]
fn asking_for_help_builds_no_context(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/context.feature",
    name = "A rejected value builds no context"
)]
#[test]
fn a_rejected_value_builds_no_context(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/context.feature",
    name = "A declined question builds no context"
)]
#[test]
fn a_declined_question_builds_no_context(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/context.feature",
    name = "A context that cannot be built stops the run"
)]
#[test]
fn a_context_that_cannot_be_built_stops_the_run(world: World) {
    let _ = world;
}

#[scenario(
    path = "features/context.feature",
    name = "A question offers the store's records as choices"
)]
#[test]
fn a_question_offers_the_stores_records(world: World) {
    let _ = world;
}
