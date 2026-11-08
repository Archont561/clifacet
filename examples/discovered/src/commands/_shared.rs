//! Shared by this folder's commands: a helper, not a command.
//!
//! It starts with `_` so discovery never registers it. With a list, nothing would declare it
//! at all; with discovery, the folder does — which is why a helper can be a file of its own
//! rather than something each command repeats.

/// The greeting this example's commands begin with.
#[must_use]
pub fn greeting() -> &'static str {
    "hello"
}
