//! A friendly wrapper around the Backlog.md CLI, built on the published API.
//!
//! The point of this crate is not to reimplement Backlog.md. It is to show what a
//! *thin* layer over somebody else's CLI costs when the layer is built on clifacet:
//! typed input ([`Request`]) instead of a stringly argument soup, a decision that is
//! data ([`Invocation`]) instead of a side effect, questions asked only when something
//! can answer them, and a run that can be printed before it happens.
//!
//! ```text
//! bl                          # list, the thing you do most
//! bl --json show CLI-20       # one --json, not one per subcommand
//! bl new "Ship it" --ac one   # the title is optional; you are asked
//! bl done CLI-20              # confirmed, and --yes skips the question
//! bl --dry-run done CLI-20    # prints the exact command instead of running it
//! ```
//!
//! Every leaf builds an argument vector and never touches the process; [`run`] is the
//! one place that spawns anything. That split is what lets the tests assert the
//! command line byte for byte. [`Backlog`] is the wrapper as a clifacet
//! [`Planned`] command: `--dry-run` prints its plan and a real run executes the same
//! plan, which the library makes the only execution path — so the preview is honest
//! rather than a separate, slowly-drifting code path.

use std::fmt;
use std::io::Write;
use std::process::Command as Process;

use clap::Parser;
use clifacet::Facet;
use clifacet::command::{Plan, Planned};
use clifacet::contract::ExitPolicy;
use clifacet::failure::Failure;
use clifacet::policy::{Capabilities, Mode, Policy, Request as ModeRequest};
use clifacet::prompt::{Collect, Confirmation, Prompter, Session, Terminal};

/// The program the wrapper runs, overridable so a test never spawns the real tool.
///
/// The value is a command with whitespace-separated arguments — `backlog` by
/// default, `bun x --bun --no-install backlog` in a repository that runs the CLI
/// through Bun. Nothing is shell-expanded, so a path containing a space needs a
/// wrapper script rather than quoting; the honest cost of not shelling out.
pub const BIN_ENV: &str = "CLIFACET_BACKLOG_BIN";

/// Where a task can be in this project's configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Not started.
    ToDo,
    /// Being worked on.
    InProgress,
    /// Finished.
    Done,
}

impl Status {
    /// Every status, in the order the tool lists them.
    pub const ALL: [Self; 3] = [Self::ToDo, Self::InProgress, Self::Done];

    /// The spelling Backlog.md itself accepts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ToDo => "To Do",
            Self::InProgress => "In Progress",
            Self::Done => "Done",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for Status {
    type Err = String;

    /// Accepts `todo`, `to-do`, `To Do` and `inprogress` alike, and says what it knows.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalised: String = value
            .chars()
            .filter(|character| character.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
        match normalised.as_str() {
            "todo" => Ok(Self::ToDo),
            "inprogress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            _ => Err(format!(
                "`{value}` is not a status; expected one of: To Do, In Progress, Done"
            )),
        }
    }
}

/// The wrapper's own command line.
///
/// The global flags are global on purpose: Backlog.md repeats `--json` on every leaf,
/// so a user who wants one machine-readable call has to remember which level it sits
/// at. Here it is accepted anywhere and forwarded where it belongs. `--dry-run` and
/// `--yes` decide how the run goes rather than what it asks for, so they stay out of
/// the [`Request`].
#[clifacet::command(input = Request)]
#[derive(Debug)]
#[command(
    name = "bl",
    version,
    about = "A friendlier front door to the Backlog.md CLI",
    long_about = "A friendlier front door to the Backlog.md CLI.\n\n\
                  Everything is forwarded to `backlog`; nothing is reimplemented. \
                  What this adds is the three things everyone retypes: a status \
                  shortcut, one --json for the whole run, and --dry-run."
)]
pub struct Cli {
    /// Print versioned machine-readable JSON instead of the human output.
    #[clifacet(long)]
    #[arg(global = true)]
    pub json: bool,

    /// Print the command that would run, and run nothing.
    #[clifacet(long, skip)]
    #[arg(global = true)]
    pub dry_run: bool,

    /// Answer confirmations with yes.
    #[clifacet(long, skip)]
    #[arg(global = true)]
    pub yes: bool,

    /// The subcommand; none means `list`.
    #[clifacet(subcommands)]
    pub command: Option<Verb>,
}

/// The wrapper's tree, declared where its leaves live (decision-14).
///
/// Adding a leaf is one variant here, one struct below and one arm in
/// [`invocation_of`] — and a missing [`Action`] variant is a compile error, not a
/// forgotten arm. The conversion, the questions and their order are generated.
#[clifacet::subcommands(input = Action)]
#[derive(Debug)]
pub enum Verb {
    /// List tasks, grouped by status.
    List(ListArgs),
    /// Show one task.
    Show(Show),
    /// Create a task. The title is asked for when it is missing.
    New(NewArgs),
    /// Move a task to In Progress.
    Start(Start),
    /// Move a task to Done.
    Done(Done),
    /// Search tasks, documents and decisions.
    Search(Search),
    /// Check the backlog for duplicate IDs and broken dependencies.
    Doctor(Doctor),
}

/// Filters for `bl list`, named the way a person asks the question.
#[clifacet::command(input = Filter, bridge = filter, collect = choose_status)]
#[derive(Debug, Default)]
pub struct ListArgs {
    /// Only tasks in this status; with no value, it is asked for as a choice.
    #[clifacet(long, value_parser = parse_status)]
    pub status: Option<Option<Status>>,

    /// Only tasks whose dependencies are all done.
    #[clifacet(long)]
    pub ready: bool,

    /// Print at most this many.
    #[clifacet(long)]
    pub limit: Option<u32>,
}

/// `bl show`.
#[clifacet::command(input = TaskRef)]
#[derive(Debug)]
pub struct Show {
    /// The task ID, for example CLI-20.
    #[clifacet(trim, non_empty)]
    pub id: String,
}

/// The fields of a new task.
#[clifacet::command(input = NewTask, bridge = new_task, collect = ask_title)]
#[derive(Debug)]
pub struct NewArgs {
    /// The title. Omitted, it is asked for; with nothing able to answer, the run fails.
    pub title: Option<String>,

    /// An acceptance criterion; repeat for more.
    #[clifacet(long = "ac")]
    pub acceptance: Vec<String>,

    /// The milestone ID or title.
    #[clifacet(long)]
    pub milestone: Option<String>,

    /// The status the task starts in.
    #[clifacet(long, value_parser = parse_status)]
    pub status: Option<Status>,
}

/// `bl start`: confirmed before anything moves.
#[clifacet::command(input = TaskRef, collect = confirm_start)]
#[derive(Debug)]
pub struct Start {
    /// The task ID.
    #[clifacet(trim, non_empty)]
    pub id: String,
}

/// `bl done`: confirmed before anything moves.
#[clifacet::command(input = TaskRef, collect = confirm_done)]
#[derive(Debug)]
pub struct Done {
    /// The task ID.
    #[clifacet(trim, non_empty)]
    pub id: String,
}

/// `bl search`.
#[clifacet::command(input = Query)]
#[derive(Debug)]
pub struct Search {
    /// What to look for.
    #[clifacet(trim, non_empty)]
    pub query: String,
}

/// `bl doctor`, which takes nothing.
#[clifacet::command(input = Checkup)]
#[derive(Debug)]
pub struct Doctor {}

fn parse_status(value: &str) -> Result<Status, String> {
    value.parse()
}

/// A validated instruction: everything the wrapper decided, and nothing it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Whether to ask for machine-readable output.
    pub json: bool,
    /// What to ask the wrapped CLI for; none means `list`, which [`invocation_of`] picks.
    pub command: Option<Action>,
}

/// What a [`Request`] asks for: one variant per leaf of [`Verb`], every value validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// List tasks.
    List(Filter),
    /// Show one task.
    Show(TaskRef),
    /// Create one task.
    New(NewTask),
    /// Move a task to In Progress.
    Start(TaskRef),
    /// Move a task to Done.
    Done(TaskRef),
    /// Search the backlog.
    Search(Query),
    /// Check the backlog's health.
    Doctor(Checkup),
}

/// Which tasks to list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// A status filter.
    pub status: Option<Status>,
    /// Only unblocked tasks.
    pub ready: bool,
    /// A display limit.
    pub limit: Option<u32>,
}

/// One task, by ID: never blank, already trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRef {
    /// The task ID.
    pub id: String,
}

/// A task to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTask {
    /// The title, never empty: it was asked for or supplied.
    pub title: String,
    /// The acceptance criteria.
    pub acceptance: Vec<String>,
    /// The milestone, if one was named.
    pub milestone: Option<String>,
    /// The status the task starts in.
    pub status: Option<Status>,
}

/// A search, never blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The query.
    pub query: String,
}

/// A health check, which needs nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkup {}

/// The exit codes this application gives its outcomes.
pub const EXIT: ExitPolicy = ExitPolicy::standard();

/// The task ID check every mutating leaf shares.
///
/// # Errors
///
/// Returns [`Failure::validation`] when the ID is blank.
pub fn task_id(id: &str) -> Result<String, Failure> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(Failure::validation("a task ID is required"));
    }
    Ok(trimmed.to_owned())
}

/// `bl list`'s input: a bare `--status` is refused unless it was asked about.
fn filter(
    ListArgs {
        status,
        ready,
        limit,
    }: ListArgs,
) -> Result<Filter, Failure> {
    let status = status
        .map(|named| named.ok_or_else(unnamed_status))
        .transpose()?;
    Ok(Filter {
        status,
        ready,
        limit,
    })
}

/// Asks which status a bare `--status` meant; still bare when nothing may ask.
fn choose_status(mut args: ListArgs, session: &mut Session) -> Result<ListArgs, Failure> {
    if args.status == Some(None) {
        args.status = Some(session.choose(None, "Which status?", Status::ALL)?);
    }
    Ok(args)
}

/// The refusal for a bare `--status` nothing could ask about.
fn unnamed_status() -> Failure {
    let statuses: Vec<&str> = Status::ALL.map(Status::as_str).into();
    Failure::validation(format!(
        "--status needs a value here, because nothing can ask which: one of {}",
        statuses.join(", ")
    ))
}

/// `bl new`'s input. A blank title is the one thing that has to be caught here rather
/// than by the wrapped tool, because the tool would happily create an untitled task.
fn new_task(args: NewArgs) -> Result<NewTask, Failure> {
    let Some(title) = args.title else {
        return Err(Failure::validation(
            "a task title is required: pass one, or run somewhere it can be asked for",
        ));
    };
    let title = title.trim().to_owned();
    if title.is_empty() {
        return Err(Failure::validation("a task title must not be blank"));
    }
    Ok(NewTask {
        title,
        acceptance: args.acceptance,
        milestone: args.milestone,
        status: args.status,
    })
}

/// Asks for the title a bare `bl new` did not get; a positional cannot be `ask`ed.
fn ask_title(mut args: NewArgs, session: &mut Session) -> Result<NewArgs, Failure> {
    args.title = session.value(args.title, "Task title?")?;
    Ok(args)
}

/// The `new` leaf on its own: asks for what is missing, then validates.
///
/// # Errors
///
/// Returns [`Failure::validation`] when nothing may ask and no title was supplied,
/// and whatever the frontend reports for a cancelled or broken question.
pub fn collect_new(args: NewArgs, session: &mut Session) -> Result<Request, Failure> {
    let task = args.collect(session).and_then(Facet::validate)?;
    Ok(Request {
        json: false,
        command: Some(Action::New(task)),
    })
}

fn confirm_start(start: Start, session: &mut Session) -> Result<Start, Failure> {
    confirm_status(&task_id(&start.id)?, Status::InProgress, session)?;
    Ok(start)
}

fn confirm_done(done: Done, session: &mut Session) -> Result<Done, Failure> {
    confirm_status(&task_id(&done.id)?, Status::Done, session)?;
    Ok(done)
}

/// Confirms a state change, or fails without running anything.
///
/// A confirmation that cannot be asked is **not** an answer: when nothing can ask and
/// `--yes` was not passed, this refuses instead of proceeding. `--yes` is the way to
/// say so in advance, and it is the only thing `--yes` decides — with or without a
/// terminal.
///
/// # Errors
///
/// Returns [`Failure::cancelled`] when the user declines,
/// [`Failure::validation`] naming `--yes` when nobody could be asked, and the
/// frontend's failure when the question breaks.
pub fn confirm_status(id: &str, status: Status, session: &mut Session) -> Result<(), Failure> {
    match session.confirm(&format!("Move {id} to {status}?"))? {
        Confirmation::Confirmed | Confirmation::Assumed => Ok(()),
        Confirmation::Declined => Err(Failure::cancelled(format!(
            "declined: {id} was not moved to {status}"
        ))),
        // Nobody could be asked, so nobody said no: the mistake was omitting `--yes`.
        Confirmation::Unavailable => Err(Failure::validation(format!(
            "refusing to move {id} to {status} unconfirmed: pass --yes to say so in advance"
        ))),
    }
}

/// The command line this request stands for, plus why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    argv: Vec<String>,
    explain: String,
}

impl Invocation {
    /// The argument vector, starting with the program.
    #[must_use]
    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// One sentence saying what the command does, for `--dry-run` to print.
    #[must_use]
    pub fn explain(&self) -> &str {
        &self.explain
    }

    /// The command as a line that can be pasted into a shell.
    ///
    /// Quoting is deliberately minimal — single quotes only for arguments that
    /// contain whitespace or a quote — so the printed line stays readable.
    #[must_use]
    pub fn shell_line(&self) -> String {
        self.argv
            .iter()
            .map(|argument| {
                if argument.is_empty()
                    || argument
                        .chars()
                        .any(|character| character.is_whitespace() || character == '\'')
                {
                    format!("'{}'", argument.replace('\'', r"'\''"))
                } else {
                    argument.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn push_args(argv: &mut Vec<String>, values: &[&str]) {
    argv.extend(values.iter().map(|value| (*value).to_owned()));
}

/// Turns a validated request into the command it stands for.
///
/// This is a pure function, and that is the design: the same value can be printed
/// (`--dry-run`), asserted in a test, or executed, and the three cannot disagree.
#[must_use]
pub fn invocation_of(request: Request) -> Invocation {
    let mut argv = program();
    let action = request
        .command
        .unwrap_or_else(|| Action::List(Filter::default()));
    let explain = match action {
        Action::List(Filter {
            status,
            ready,
            limit,
        }) => {
            push_args(&mut argv, &["task", "list"]);
            if let Some(status) = status {
                argv.push(String::from("--status"));
                argv.push(status.as_str().to_owned());
            }
            if ready {
                argv.push(String::from("--ready"));
            }
            if let Some(limit) = limit {
                argv.push(String::from("--limit"));
                argv.push(limit.to_string());
            }
            String::from("list tasks")
        }
        Action::Show(TaskRef { id }) => {
            push_args(&mut argv, &["task", "view"]);
            argv.push(id.clone());
            if !request.json {
                argv.push(String::from("--plain"));
            }
            format!("show {id}")
        }
        Action::New(NewTask {
            title,
            acceptance,
            milestone,
            status,
        }) => {
            push_args(&mut argv, &["task", "create"]);
            argv.push(title.clone());
            for criterion in &acceptance {
                argv.push(String::from("--ac"));
                argv.push(criterion.clone());
            }
            if let Some(milestone) = &milestone {
                argv.push(String::from("--milestone"));
                argv.push(milestone.clone());
            }
            if let Some(status) = status {
                argv.push(String::from("--status"));
                argv.push(status.as_str().to_owned());
            }
            format!("create a task titled \u{201c}{title}\u{201d}")
        }
        Action::Start(TaskRef { id }) => set_status(&mut argv, &id, Status::InProgress),
        Action::Done(TaskRef { id }) => set_status(&mut argv, &id, Status::Done),
        Action::Search(Query { query }) => {
            push_args(&mut argv, &["search"]);
            argv.push(query.clone());
            format!("search for \u{201c}{query}\u{201d}")
        }
        Action::Doctor(Checkup {}) => {
            push_args(&mut argv, &["doctor"]);
            String::from("check the backlog for duplicate IDs and broken dependencies")
        }
    };
    if request.json {
        argv.push(String::from("--json"));
    }
    Invocation { argv, explain }
}

/// `backlog task edit ID --status S`, and the sentence for it.
fn set_status(argv: &mut Vec<String>, id: &str, status: Status) -> String {
    push_args(argv, &["task", "edit", id, "--status", status.as_str()]);
    format!("move {id} to {status}")
}

/// The command to run: the environment's override, or `backlog`.
#[must_use]
pub fn program() -> Vec<String> {
    std::env::var(BIN_ENV).map_or_else(
        |_| vec![String::from("backlog")],
        |value| {
            let parts: Vec<String> = value.split_whitespace().map(str::to_owned).collect();
            if parts.is_empty() {
                vec![String::from("backlog")]
            } else {
                parts
            }
        },
    )
}

/// Runs an invocation, inheriting the terminal, and answers with its exit status.
///
/// The wrapped tool owns its own exit codes, and its own output; this returns the
/// child's status unchanged instead of remapping it, because remapping somebody
/// else's contract is how a wrapper starts lying. The wrapper's *own* failures are
/// mapped by the application, which is the one thing clifacet does decide.
///
/// # Errors
///
/// Returns [`Failure::execution`] when the program cannot be spawned at all — a
/// missing `backlog`, usually — and when the child ends without an exit code.
pub fn run(invocation: &Invocation) -> Result<i32, Failure> {
    let (program, arguments) = invocation
        .argv()
        .split_first()
        .ok_or_else(|| Failure::execution("nothing to run"))?;
    let status = Process::new(program)
        .args(arguments)
        .status()
        .map_err(|error| Failure::execution(format!("cannot run {program}: {error}")))?;
    status
        .code()
        .ok_or_else(|| Failure::execution(format!("{program} ended without an exit code")))
}

/// Runs one invocation and answers with the status the process should exit with.
///
/// Capabilities and the frontend are parameters rather than lookups, so a test can
/// run the whole application — including the questions — without a terminal.
pub fn app(
    cli: Cli,
    capabilities: Capabilities,
    build_frontend: impl FnOnce() -> Box<dyn Prompter>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let requested = if capabilities.supports(Mode::Interactive) {
        ModeRequest::mode(Mode::Interactive)
    } else {
        ModeRequest::none()
    };
    let plan = match Policy::up_to(Mode::Interactive)
        .resolve(requested.assuming_yes(cli.yes), capabilities)
    {
        Ok(plan) => plan,
        Err(refusal) => {
            report(err, &refusal.message());
            return EXIT.execution();
        }
    };
    let dry_run = cli.dry_run;
    let mut session = Session::start(plan, build_frontend);
    // The chosen leaf asks its own questions, then validates by its own rules; an
    // absent subcommand asks nothing and reaches `invocation_of` as `None`.
    let planned = cli
        .collect(&mut session)
        .and_then(Facet::validate)
        .and_then(|request| Backlog.plan(request));
    // The dry run describes the plan and the real run executes it: one value, so the
    // printed line is the line that runs.
    let outcome = match planned {
        Ok(plan) if dry_run => {
            report(out, &plan.describe());
            return EXIT.success();
        }
        Ok(plan) => plan.execute(),
        Err(failure) => Err(failure),
    };
    match outcome {
        Ok(code) => code,
        Err(failure) => clifacet::report::render("bl", &failure, &EXIT, out, err),
    }
}

/// The wrapper as a clifacet command: a request is planned as one invocation of the
/// wrapped tool, and running it is executing that plan.
///
/// The plan's steps are the sentence and the runnable line `--dry-run` prints; its work
/// is [`run`]. Because the library implements [`clifacet::Command`] for a [`Planned`]
/// command as "plan, then execute", there is no other way to run one.
#[derive(Debug, Clone, Copy, Default)]
pub struct Backlog;

impl Planned for Backlog {
    type Input = Request;
    /// The wrapped tool's own exit status, passed through unchanged.
    type Output = i32;

    fn plan(&self, request: Request) -> Result<Plan<'_, i32>, Failure> {
        let invocation = invocation_of(request);
        let steps = [
            invocation.explain().to_owned(),
            format!("    {}", invocation.shell_line()),
        ];
        let plan = Plan::new(move || run(&invocation));
        Ok(steps.into_iter().fold(plan, Plan::step))
    }
}

/// Writes one line, ignoring a closed stream: a broken pipe is not a failure to report.
fn report(stream: &mut dyn Write, line: &str) {
    let _ = writeln!(stream, "{line}");
}

/// The frontend a real process uses: the library's dialog, on the process terminal.
#[must_use]
pub fn terminal_frontend() -> Box<dyn Prompter> {
    Box::new(Terminal::stdio())
}

/// The entry point a real binary calls.
#[must_use]
pub fn main(
    argv: impl IntoIterator<Item = std::ffi::OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match Cli::try_parse_from(argv) {
        Ok(cli) => app(cli, Capabilities::detected(), terminal_frontend, out, err),
        // Help is the result the user asked for, on the data stream; a usage error is
        // a diagnostic. `render` applies that, with the codes `EXIT` gives them.
        Err(error) => clifacet::report::render("bl", &error, &EXIT, out, err),
    }
}
