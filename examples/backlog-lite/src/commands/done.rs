//! `bl done`: move a task to Done, confirmed before anything moves.

use clifacet::failure::Failure;
use clifacet::prompt::Session;

use super::_task::{self, TaskRef};
use crate::status::Status;

/// Move a task to Done.
///
/// Confirmed before anything moves, for the same reason `start` is: a state change nobody
/// agreed to is not one to make.
#[clifacet::command(input = TaskRef, collect = confirm)]
pub fn done(#[clifacet(trim, non_empty)] id: String) -> Result<TaskRef, Failure> {
    Ok(TaskRef {
        id: _task::task_id(&id)?,
    })
}

/// Confirms before anything moves.
fn confirm(done: Done, session: &mut Session) -> Result<Done, Failure> {
    _task::confirm_status(&_task::task_id(&done.id)?, Status::Done, session)?;
    Ok(done)
}

/// `backlog task edit ID --status Done`, and the sentence for it.
pub(crate) fn forward(TaskRef { id }: TaskRef, argv: &mut Vec<String>) -> String {
    _task::set_status(argv, &id, Status::Done)
}
