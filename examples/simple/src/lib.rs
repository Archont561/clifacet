//! The reference application: one domain-neutral command, end to end.
//!
//! This crate is the worked example M1 asks for, and the shape an application is
//! meant to copy. It depends on `clifacet` the way any application does — through
//! the published API and nothing else — which is what makes it a proof of that API
//! rather than a part of it. It holds the three layers apart on purpose, and the command
//! is one function:
//!
//! 1. **the declaration** is the parameter list of [`example`], each argument declared once
//!    with `#[clifacet(..)]`: its grammar, its question, its normalisation and its place on
//!    the form (decision-13). The doc comment is the command's description.
//! 2. [`ExampleInput`] is validated input, generated from those same declarations by
//!    `#[clifacet::command]`, which ends in [`check_input`] for the one rule that is the
//!    application's own. Everything that can be wrong with a value is rejected there, so
//!    the input is total.
//! 3. [`Example`] is the command — the macro's name for the leaf — generated beside the
//!    function: it parses (`clap`'s grammar, unchanged), validates, and calls the function.
//!    The body touches no stream and knows nothing about `clap`.
//!
//! [`app`] is the application layer that joins them: one [`clifacet::App`] that
//! parses, settles the interaction mode against [`POLICY`], asks, validates, runs,
//! writes machine data to one stream and diagnostics to the other, and maps failures to
//! the exit codes in [`EXIT`]. None of that lives in [`Command`], which is what keeps
//! the seam UI-free, and none of it is written here: it is the library's runner.
//!
//! # Layout
//!
//! One file per command, and one file per concern of the application around it:
//!
//! ```text
//! src/commands/mod.rs      the registry: one `mod` line per command
//! src/commands/example.rs  the command: `example`, `ExampleInput`, `Example`
//! src/interaction.rs       the interaction flags and the mode `POLICY`
//! src/exit.rs              the exit codes, and the one place an outcome is written
//! src/entry.rs             the application as one `App`, and the entry points
//! src/form.rs              the full-screen form, driven by a script of keys
//! ```
//!
//! Everything public is re-exported here, so a caller writes `clifacet_example::Cli`
//! and never needs to know which file it lives in.
//!
//! [`Command`]: clifacet::Command
//! [`Input`]: clifacet::Command::Input

mod commands;
mod entry;
mod exit;
mod form;
mod interaction;

pub use commands::{
    Example, ExampleInput, ExampleOutput, Level, MAX_REPEAT, check_input, example_command,
};
pub use entry::{
    app, dialog_frontend, main, main_with, main_with_command, run, run_with, run_with_command,
    terminal_frontend,
};
pub use exit::{
    EXIT, EXIT_CANCELLED, EXIT_EXECUTION_FAILED, EXIT_SUCCESS, EXIT_USAGE, EXIT_VALIDATION_FAILED,
    exit_code,
};
pub use form::{run_tui, run_tui_with, tui_form, tui_submit, tui_submit_with};
pub use interaction::{InteractionFlags, POLICY};
