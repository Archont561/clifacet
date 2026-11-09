//! `bl search`: one query across tasks, documents and decisions.

use clifacet::failure::Failure;

/// Search everything the backlog holds.
///
/// One query, matched across tasks, documents and decisions.
#[clifacet::command(input = Query)]
pub fn search(#[clifacet(trim, non_empty)] query: String) -> Result<Query, Failure> {
    Ok(Query { query })
}

/// A search, never blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// The query.
    pub query: String,
}

/// `backlog search QUERY`, and the sentence for it.
pub(crate) fn forward(Query { query }: Query, argv: &mut Vec<String>) -> String {
    crate::invocation::push_args(argv, &["search"]);
    argv.push(query.clone());
    format!("search for \u{201c}{query}\u{201d}")
}
