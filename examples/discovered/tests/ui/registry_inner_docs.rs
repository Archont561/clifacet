//! A folder whose `mod.rs` starts with `//!` (CLI-58).
//!
//! A registry pulls the folder in with `include!`, and an included fragment may not begin with
//! inner attributes or `//!` docs. Rust's own message for that is about the fragment and never
//! mentions the attribute, so the macro reads the header first and says what to do instead.
#[clifacet::registry(output = String)]
mod commands;

fn main() {}
