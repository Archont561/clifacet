//! The application's half of discovery (decision-19 item 4, CLI-50).
//!
//! One call. It scans `src/commands/`, tells Cargo to rebuild when that folder changes, and
//! hands the result to `clifacet::commands!()` through the environment — because a build
//! script and a macro in the same crate have no other channel between them.

fn main() {
    clifacet::build::commands();
}
