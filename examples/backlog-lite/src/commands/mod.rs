clifacet::__commands!(
    doctor,
    done,
    list,
    new,
    search,
    show,
    start,
    root = { "json": bool },
    request = Action,
    name = "bl",
    about = "A friendlier front door to the Backlog.md CLI",
    long_about = "A friendlier front door to the Backlog.md CLI.\n\n\
                  Everything is forwarded to `backlog`; nothing is reimplemented. \
                  What this adds is the three things everyone retypes: a status \
                  shortcut, one --json for the whole run, and --dry-run."
);

mod _task;

pub use _task::{TaskRef, confirm_status, task_id};
pub use doctor::Checkup;
pub use list::Filter;
pub use new::{NewTask, collect_new};
pub use search::Query;

/// A validated instruction: everything the wrapper decided, and nothing it did.
///
/// The registry generated one part of it — [`Action`], one variant per leaf, each holding
/// that leaf's validated input — and the root's own `json` flag is the other. Keeping them
/// apart is what lets the flag stay global and the request stay one value.
#[derive(Debug, Clone)]
pub struct Request {
    /// Whether to ask for machine-readable output.
    pub json: bool,
    /// What to ask the wrapped CLI for; none means `list`, which
    /// [`invocation_of`](crate::invocation_of) picks.
    pub command: Option<Action>,
}

pub(crate) use doctor::forward as forward_doctor;
pub(crate) use done::forward as forward_done;
pub(crate) use list::forward as forward_list;
pub(crate) use new::forward as forward_new;
pub(crate) use search::forward as forward_search;
pub(crate) use show::forward as forward_show;
pub(crate) use start::forward as forward_start;

impl From<Input> for Request {
    /// What the root's input means: the chosen command, and the root's own flag.
    ///
    /// A bare `bl` means `list` — the application's decision, not the parser's — so
    /// [`invocation_of`](crate::invocation_of) is handed a request that always names one.
    fn from(input: Input) -> Self {
        Self {
            json: input.json,
            command: input.command,
        }
    }
}
