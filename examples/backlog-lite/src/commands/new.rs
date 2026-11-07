//! `bl new`: create a task, asking for the title when it was not given.

use clifacet::Facet;
use clifacet::failure::Failure;
use clifacet::prompt::{Collect, Session};

use super::{Action, Request};
use crate::status::{Status, parse_status};

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

/// `backlog task create TITLE` with its options, and the sentence for it.
pub(crate) fn forward(
    NewTask {
        title,
        acceptance,
        milestone,
        status,
    }: NewTask,
    argv: &mut Vec<String>,
) -> String {
    crate::invocation::push_args(argv, &["task", "create"]);
    argv.push(title.clone());
    for criterion in acceptance {
        argv.push(String::from("--ac"));
        argv.push(criterion);
    }
    if let Some(milestone) = milestone {
        argv.push(String::from("--milestone"));
        argv.push(milestone);
    }
    if let Some(status) = status {
        argv.push(String::from("--status"));
        argv.push(status.as_str().to_owned());
    }
    format!("create a task titled \u{201c}{title}\u{201d}")
}
