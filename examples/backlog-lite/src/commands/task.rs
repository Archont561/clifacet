//! What the leaves that name one task share: the reference, its check, and moving it.
//!
//! Not a command: `show`, `start` and `done` all take a [`TaskRef`], and the two that
//! change a task's status confirm and forward the same way.

use clifacet::failure::Failure;
use clifacet::prompt::{Confirmation, Session};

use crate::status::Status;

/// One task, by ID: never blank, already trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRef {
    /// The task ID.
    pub id: String,
}

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

/// `backlog task edit ID --status S`, and the sentence for it.
pub(crate) fn set_status(argv: &mut Vec<String>, id: &str, status: Status) -> String {
    crate::invocation::push_args(argv, &["task", "edit", id, "--status", status.as_str()]);
    format!("move {id} to {status}")
}
