//! `bl start`: move a task to In Progress, confirmed before anything moves.

use clifacet::failure::Failure;
use clifacet::prompt::Session;

use super::task::{self, TaskRef};
use crate::status::Status;

/// `bl start`: confirmed before anything moves.
#[clifacet::command(input = TaskRef, collect = confirm_start)]
#[derive(Debug, Clone)]
pub struct Start {
    /// The task ID.
    #[clifacet(trim, non_empty)]
    pub id: String,
}

fn confirm_start(start: Start, session: &mut Session) -> Result<Start, Failure> {
    task::confirm_status(&task::task_id(&start.id)?, Status::InProgress, session)?;
    Ok(start)
}

/// `backlog task edit ID --status "In Progress"`, and the sentence for it.
pub(crate) fn forward(TaskRef { id }: TaskRef, argv: &mut Vec<String>) -> String {
    task::set_status(argv, &id, Status::InProgress)
}
