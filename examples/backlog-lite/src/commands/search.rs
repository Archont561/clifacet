//! `bl search`: one query across tasks, documents and decisions.

/// `bl search`.
#[clifacet::command(input = Query)]
#[derive(Debug)]
pub struct Search {
    /// What to look for.
    #[clifacet(trim, non_empty)]
    pub query: String,
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
