//! `bl start`: move a task to In Progress, confirmed before anything moves.

use clifacet::failure::Failure;
use clifacet::prompt::Session;

use super::_task::{self, TaskRef};
use crate::status::Status;

/// Move a task to In Progress.
///
/// A confirmation that cannot be asked is not an answer, so the leaf refuses instead of moving
/// the task: `collect = confirm` is where that rule lives, and the generated command keeps it.
#[clifacet::command(input = TaskRef, collect = confirm)]
pub fn start(#[clifacet(trim, non_empty)] id: String) -> Result<TaskRef, Failure> {
    Ok(TaskRef {
        id: _task::task_id(&id)?,
    })
}

/// Confirms before anything moves.
fn confirm(start: Start, session: &mut Session) -> Result<Start, Failure> {
    _task::confirm_status(&_task::task_id(&start.id)?, Status::InProgress, session)?;
    Ok(start)
}

/// `backlog task edit ID --status "In Progress"`, and the sentence for it.
pub(crate) fn forward(TaskRef { id }: TaskRef, argv: &mut Vec<String>) -> String {
    _task::set_status(argv, &id, Status::InProgress)
}
