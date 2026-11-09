//! A registry written on a module that is not the folder (CLI-58).
//!
//! The attribute writes the registration *into* the module it is given, so that module is the
//! folder and its name is how the folder is found. A module called anything else has no
//! registration to receive, and the message says what to write instead.
#[clifacet::registry(name = "discovered")]
mod store {}

fn main() {}
