// `_task` is the helper `show`, `start` and `done` share: a task ID and moving it. Its `mod`
// line is the registry's — it declares the folder's helpers beside its leaves — so it is not
// written here, and this file only re-exports what it makes reachable.

pub use doctor::Checkup;
pub use list::Filter;
pub use new::{NewTask, collect_new};
pub use search::Query;
// `_task`'s items are already in this module — the registry declared it here, and the leaves
// reach it as `super::_task` — so the re-export is what makes them the folder's public surface.
pub use _task::{TaskRef, confirm_status, task_id};

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
