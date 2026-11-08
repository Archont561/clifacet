//! `bl list`: the command people run most, so it is also what a bare `bl` runs.

use clifacet::failure::Failure;
use clifacet::prompt::Session;

use crate::status::{Status, parse_status};

/// Filters for `bl list`, named the way a person asks the question.
#[clifacet::command(input = Filter, bridge = filter, collect = choose_status)]
#[derive(Debug, Clone, Default)]
pub struct List {
    /// Only tasks in this status; with no value, it is asked for as a choice.
    //
    // `Option<Option<_>>` on purpose: `Some(None)` is "the flag was written bare", which
    // is a question to ask, and `None` is "no filter". Collapsing it would lose the
    // difference between the user asking to be asked and the user saying nothing.
    #[allow(clippy::option_option)]
    #[clifacet(long, value_parser = parse_status)]
    pub status: Option<Option<Status>>,

    /// Only tasks whose dependencies are all done.
    #[clifacet(long)]
    pub ready: bool,

    /// Print at most this many.
    #[clifacet(long)]
    pub limit: Option<u32>,
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

/// `bl list`'s input: a bare `--status` is refused unless it was asked about.
fn filter(
    List {
        status,
        ready,
        limit,
    }: List,
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
fn choose_status(mut args: List, session: &mut Session) -> Result<List, Failure> {
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

/// `backlog task list` with the filters that were given, and the sentence for it.
pub(crate) fn forward(
    Filter {
        status,
        ready,
        limit,
    }: Filter,
    argv: &mut Vec<String>,
) -> String {
    crate::invocation::push_args(argv, &["task", "list"]);
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
