//! `bl done`: move a task to Done, confirmed before anything moves.

use clifacet::failure::Failure;
use clifacet::prompt::Session;

use super::task::{self, TaskRef};
use crate::status::Status;

/// `bl done`: confirmed before anything moves.
#[clifacet::command(input = TaskRef, collect = confirm_done)]
#[derive(Debug, Clone)]
pub struct Done {
    /// The task ID.
    #[clifacet(trim, non_empty)]
    pub id: String,
}

fn confirm_done(done: Done, session: &mut Session) -> Result<Done, Failure> {
    task::confirm_status(&task::task_id(&done.id)?, Status::Done, session)?;
    Ok(done)
}

/// `backlog task edit ID --status Done`, and the sentence for it.
pub(crate) fn forward(TaskRef { id }: TaskRef, argv: &mut Vec<String>) -> String {
    task::set_status(argv, &id, Status::Done)
}
