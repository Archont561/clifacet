//! The map of the CLI: the root, its tree, and the request they validate into.
//!
//! One file per leaf, and this file is the only place they are registered — a single
//! `clifacet::commands!` call, whose list *is* the registry (decision-19 item 3, CLI-33).
//! It declares the modules, their re-exports, the [`Commands`] tree, [`Request`], [`Input`]
//! and the root [`Cli`]: what used to be three hand-kept lists that had to be kept in step
//! with the files beside them.
//!
//! A leaf file holds everything about its command: the `clap` struct, the validated
//! input, its own bridge and questions, and the arguments it forwards. Adding a leaf is a
//! new file, its name in the list below, and one arm in
//! [`invocation_of`](crate::invocation_of) — which the compiler demands, because
//! [`Request`] gains a variant. The file stem is the command's name and the struct is its
//! Pascal case form, so `doctor` is `Doctor` and `bl doctor` is the subcommand; a leaf may
//! still rename itself with `#[command(name = "..")]`.
//!
//! The registry declares the modules too, so the list below is the file listing.
//!
//! The root's own flag is declared with `root = { .. }`: `json` stays global, so one
//! `--json` covers the whole run rather than one per leaf, and it travels into [`Input`]
//! beside the chosen command instead of being flattened into every leaf. `--dry-run` and
//! `--yes` come from `clifacet::Globals`, which the generated root flattens for every
//! application, and they stay out of the [`Request`]: they decide how a run goes rather
//! than what it asks for.

// What `show`, `start` and `done` share: a task ID and moving it. Not a command, so not
// in the list below.
mod task;

// The registry: one command per line, in the order the help lists them. The leaves are
// re-exported by this call, so nothing else has to name them twice. `task` is not in it:
// it is what three leaves share, not a command.
clifacet::commands!(
    list,
    show,
    new,
    start,
    done,
    search,
    doctor,
    root = { "json": bool },
    request = Action,
    name = "bl",
    about = "A friendlier front door to the Backlog.md CLI",
    long_about = "A friendlier front door to the Backlog.md CLI.\n\n\
                  Everything is forwarded to `backlog`; nothing is reimplemented. \
                  What this adds is the three things everyone retypes: a status \
                  shortcut, one --json for the whole run, and --dry-run."
);

pub use doctor::Checkup;
pub use list::Filter;
pub use new::{NewTask, collect_new};
pub use search::Query;
pub use task::{TaskRef, confirm_status, task_id};

/// A validated instruction: everything the wrapper decided, and nothing it did.
///
/// `commands!` generated one part of it — [`Action`], one variant per leaf, each holding
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
