//! `bl show`: one task, in the tool's plain output unless `--json` was asked for.

use clifacet::failure::Failure;

use super::task::{self, TaskRef};

/// Show one task.
///
/// The task ID is trimmed and refused if blank, before the wrapped tool sees it.
#[clifacet::command(input = TaskRef)]
pub fn show(#[clifacet(trim, non_empty)] id: String) -> Result<TaskRef, Failure> {
    Ok(TaskRef {
        id: task::task_id(&id)?,
    })
}

/// `backlog task view ID`, plain unless the run is JSON, and the sentence for it.
pub(crate) fn forward(TaskRef { id }: TaskRef, json: bool, argv: &mut Vec<String>) -> String {
    crate::invocation::push_args(argv, &["task", "view"]);
    argv.push(id.clone());
    if !json {
        argv.push(String::from("--plain"));
    }
    format!("show {id}")
}
