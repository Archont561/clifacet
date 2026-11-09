//! The application's half of the folder's registration (CLI-50, CLI-58).
//!
//! One call. It scans `src/commands/`, tells Cargo to rebuild when that folder changes, and
//! hands the result to `#[clifacet::registry]` through the environment — the only channel a
//! build script and a macro share.

fn main() {
    clifacet::build::commands();
}
