//! The commands this application has, one file each.
//!
//! There is one, so this registry is a single `mod` line. A second command would be a
//! second file here plus a `#[clifacet::subcommands]` tree in this file, with the
//! `Request` it validates into written next to it — `examples/backlog-lite` has
//! seven leaves laid out exactly that way.

mod example;

pub use example::{
    Example, ExampleInput, ExampleOutput, Level, MAX_REPEAT, check_input, example_command,
};
