//! A friendly wrapper around the Backlog.md CLI, built on the published API.
//!
//! The point of this crate is not to reimplement Backlog.md. It is to show what a
//! *thin* layer over somebody else's CLI costs when the layer is built on clifacet:
//! typed input ([`Request`]) instead of a stringly argument soup, a decision that is
//! data ([`Plan`]) instead of a side effect, questions asked only when something can
//! answer them, and a run that can be printed before it happens.
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
//! command line byte for byte, and what makes `--dry-run` honest rather than a
//! separate, slowly-drifting code path.

use std::fmt;
use std::io::Write;
use std::process::Command as Process;

use clap::Parser;
use clifacet::contract::ExitPolicy;
use clifacet::failure::Failure;
use clifacet::policy::{Capabilities, Mode, Policy, Request as ModeRequest};
use clifacet::prompt::{Confirmation, Prompter, Session, Terminal};

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
/// at. Here it is accepted anywhere and forwarded where it belongs.
#[derive(Debug, Parser)]
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
    #[arg(long, global = true)]
    pub json: bool,

    /// Print the command that would run, and run nothing.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Answer confirmations with yes.
    #[arg(long, global = true)]
    pub yes: bool,

    /// The subcommand; none means `list`.
    #[command(subcommand)]
    pub command: Option<Verb>,
}

/// One leaf of the wrapper's tree.
///
/// The tree is written out by hand, which is what clifacet supports today (decision-14
/// accepts a declarative spelling for it, which CLI-32 builds and moves this tree to).
/// Adding a leaf is one variant, one struct and one arm — no registry, no macro,
/// nothing global.
#[derive(Debug, clap::Subcommand)]
pub enum Verb {
    /// List tasks, grouped by status.
    List(ListArgs),
    /// Show one task.
    Show {
        /// The task ID, for example CLI-20.
        id: String,
    },
    /// Create a task. The title is asked for when it is missing.
    New(NewArgs),
    /// Move a task to In Progress.
    Start {
        /// The task ID.
        id: String,
    },
    /// Move a task to Done.
    Done {
        /// The task ID.
        id: String,
    },
    /// Search tasks, documents and decisions.
    Search {
        /// What to look for.
        query: String,
    },
    /// Check the backlog for duplicate IDs and broken dependencies.
    Doctor,
}

/// Filters for `bl list`, named the way a person asks the question.
#[derive(Debug, Default, clap::Args)]
pub struct ListArgs {
    /// Only tasks in this status; with no value, it is asked for as a choice.
    #[arg(long, value_parser = parse_status)]
    pub status: Option<Option<Status>>,

    /// Only tasks whose dependencies are all done.
    #[arg(long)]
    pub ready: bool,

    /// Print at most this many.
    #[arg(long)]
    pub limit: Option<u32>,
}

/// The fields of a new task.
#[derive(Debug, clap::Args)]
pub struct NewArgs {
    /// The title. Omitted, it is asked for; with nothing able to answer, the run fails.
    pub title: Option<String>,

    /// An acceptance criterion; repeat for more.
    #[arg(long = "ac")]
    pub acceptance: Vec<String>,

    /// The milestone ID or title.
    #[arg(long)]
    pub milestone: Option<String>,

    /// The status the task starts in.
    #[arg(long, value_parser = parse_status)]
    pub status: Option<Status>,
}

fn parse_status(value: &str) -> Result<Status, String> {
    value.parse()
}

/// A validated instruction: everything the wrapper decided, and nothing it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// What to ask the wrapped CLI for.
    pub verb: Action,
    /// Whether to ask for machine-readable output.
    pub json: bool,
}

/// What a [`Request`] asks for, with every value already validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// List tasks.
    List {
        /// A status filter.
        status: Option<Status>,
        /// Only unblocked tasks.
        ready: bool,
        /// A display limit.
        limit: Option<u32>,
    },
    /// Show one task.
    Show {
        /// The task ID.
        id: String,
    },
    /// Create one task.
    New {
        /// The title, never empty: it was asked for or supplied.
        title: String,
        /// The acceptance criteria.
        acceptance: Vec<String>,
        /// The milestone, if one was named.
        milestone: Option<String>,
        /// The status the task starts in.
        status: Option<Status>,
    },
    /// Move a task to another status.
    SetStatus {
        /// The task ID.
        id: String,
        /// Where to move it.
        status: Status,
    },
    /// Search the backlog.
    Search {
        /// The query.
        query: String,
    },
    /// Check the backlog's health.
    Doctor,
}

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

impl TryFrom<Cli> for Request {
    type Error = Failure;

    /// Validates the parsed wrapper arguments.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::validation`] for an empty task ID or an empty title; a
    /// blank title is the one thing that has to be caught here rather than by the
    /// wrapped tool, because the tool would happily create an untitled task.
    fn try_from(cli: Cli) -> Result<Self, Self::Error> {
        let json = cli.json;
        let verb = match cli
            .command
            .unwrap_or_else(|| Verb::List(ListArgs::default()))
        {
            Verb::List(args) => Action::List {
                status: args
                    .status
                    .map(|named| named.ok_or_else(unnamed_status))
                    .transpose()?,
                ready: args.ready,
                limit: args.limit,
            },
            Verb::Show { id } => Action::Show { id: task_id(&id)? },
            Verb::New(args) => {
                let title = args.title.unwrap_or_default();
                if title.trim().is_empty() {
                    return Err(Failure::validation(
                        "a task title is required: pass one, or run somewhere it can be asked for",
                    ));
                }
                Action::New {
                    title: title.trim().to_owned(),
                    acceptance: args.acceptance,
                    milestone: args.milestone,
                    status: args.status,
                }
            }
            Verb::Start { id } => Action::SetStatus {
                id: task_id(&id)?,
                status: Status::InProgress,
            },
            Verb::Done { id } => Action::SetStatus {
                id: task_id(&id)?,
                status: Status::Done,
            },
            Verb::Search { query } => {
                if query.trim().is_empty() {
                    return Err(Failure::validation("a search query is required"));
                }
                Action::Search {
                    query: query.trim().to_owned(),
                }
            }
            Verb::Doctor => Action::Doctor,
        };
        Ok(Self { verb, json })
    }
}

/// The refusal for a bare `--status` nothing could ask about.
fn unnamed_status() -> Failure {
    let statuses: Vec<&str> = Status::ALL.map(Status::as_str).into();
    Failure::validation(format!(
        "--status needs a value here, because nothing can ask which: one of {}",
        statuses.join(", ")
    ))
}

/// Asks for the title a bare `bl new` did not get.
///
/// # Errors
///
/// Returns [`Failure::validation`] when nothing may ask and no title was supplied,
/// and whatever the frontend reports for a cancelled or broken question.
pub fn collect_new(args: NewArgs, session: &mut Session) -> Result<Request, Failure> {
    let title = session.value(args.title.clone(), "Task title?")?;
    let Some(title) = title else {
        return Err(Failure::validation(
            "a task title is required: pass one, or run somewhere it can be asked for",
        ));
    };
    let title = title.trim().to_owned();
    if title.is_empty() {
        return Err(Failure::validation("a task title must not be blank"));
    }
    Ok(Request {
        verb: Action::New {
            title,
            acceptance: args.acceptance,
            milestone: args.milestone,
            status: args.status,
        },
        json: false,
    })
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
pub struct Plan {
    argv: Vec<String>,
    explain: String,
}

impl Plan {
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
pub fn plan_of(request: Request) -> Plan {
    let mut argv = program();
    let explain = match request.verb {
        Action::List {
            status,
            ready,
            limit,
        } => {
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
        Action::Show { id } => {
            push_args(&mut argv, &["task", "view"]);
            argv.push(id.clone());
            if !request.json {
                argv.push(String::from("--plain"));
            }
            format!("show {id}")
        }
        Action::New {
            title,
            acceptance,
            milestone,
            status,
        } => {
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
        Action::SetStatus { id, status } => {
            push_args(&mut argv, &["task", "edit"]);
            argv.push(id.clone());
            push_args(&mut argv, &["--status"]);
            argv.push(status.as_str().to_owned());
            format!("move {id} to {status}")
        }
        Action::Search { query } => {
            push_args(&mut argv, &["search"]);
            argv.push(query.clone());
            format!("search for \u{201c}{query}\u{201d}")
        }
        Action::Doctor => {
            push_args(&mut argv, &["doctor"]);
            String::from("check the backlog for duplicate IDs and broken dependencies")
        }
    };
    if request.json {
        argv.push(String::from("--json"));
    }
    Plan { argv, explain }
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

/// Runs a plan, inheriting the terminal, and answers with its exit status.
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
pub fn run(plan: &Plan) -> Result<i32, Failure> {
    let (program, arguments) = plan
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
    let json = cli.json;
    let dry_run = cli.dry_run;
    let mut session = Session::start(plan, build_frontend);
    match collect(cli, &mut session) {
        Ok(mut request) => {
            request.json = json;
            let planned = plan_of(request);
            if dry_run {
                report(out, planned.explain());
                report(out, &format!("    {}", planned.shell_line()));
                return EXIT.success();
            }
            match run(&planned) {
                Ok(code) => code,
                Err(failure) => clifacet::report::render("bl", &failure, &EXIT, out, err),
            }
        }
        Err(failure) => clifacet::report::render("bl", &failure, &EXIT, out, err),
    }
}

/// Collects whatever the leaves need asking for, then validates.
fn collect(cli: Cli, session: &mut Session) -> Result<Request, Failure> {
    let json = cli.json;
    let command = cli
        .command
        .unwrap_or_else(|| Verb::List(ListArgs::default()));
    match command {
        Verb::New(args) => {
            let mut request = collect_new(args, session)?;
            request.json = json;
            Ok(request)
        }
        Verb::Start { id } => {
            let id = task_id(&id)?;
            confirm_status(&id, Status::InProgress, session)?;
            Ok(Request {
                verb: Action::SetStatus {
                    id,
                    status: Status::InProgress,
                },
                json,
            })
        }
        Verb::Done { id } => {
            let id = task_id(&id)?;
            confirm_status(&id, Status::Done, session)?;
            Ok(Request {
                verb: Action::SetStatus {
                    id,
                    status: Status::Done,
                },
                json,
            })
        }
        Verb::List(mut args) => {
            if args.status == Some(None) {
                // Still `None` when nothing may ask, which `Request::try_from` refuses.
                args.status = Some(session.choose(None, "Which status?", Status::ALL)?);
            }
            Request::try_from(Cli {
                json,
                dry_run: false,
                yes: false,
                command: Some(Verb::List(args)),
            })
        }
        command => Request::try_from(Cli {
            json,
            dry_run: false,
            yes: false,
            command: Some(command),
        }),
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
