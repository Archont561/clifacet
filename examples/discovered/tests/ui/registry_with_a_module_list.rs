//! A registry that lists its modules (CLI-58).
//!
//! `clifacet::commands!(list, echo)` is the list form; a registry is the other one, where the
//! folder *is* the list, read from the build script's report. Writing a name here is the
//! mistake a reader of the list form makes first, so it is reported as a key that is not one.
#[clifacet::registry(list, echo)]
mod commands;

fn main() {}
