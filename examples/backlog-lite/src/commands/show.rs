//! `bl show`: one task, in the tool's plain output unless `--json` was asked for.

use super::task::TaskRef;

/// `bl show`.
#[clifacet::command(input = TaskRef)]
#[derive(Debug)]
pub struct Show {
    /// The task ID, for example CLI-20.
    #[clifacet(trim, non_empty)]
    pub id: String,
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
