//! `bl doctor`: the backlog's health check, which takes nothing.

/// `bl doctor`, which takes nothing.
#[clifacet::command(input = Checkup)]
#[derive(Debug)]
pub struct Doctor {}

/// A health check, which needs nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkup {}

/// `backlog doctor`, and the sentence for it.
pub(crate) fn forward(Checkup {}: Checkup, argv: &mut Vec<String>) -> String {
    crate::invocation::push_args(argv, &["doctor"]);
    String::from("check the backlog for duplicate IDs and broken dependencies")
}
