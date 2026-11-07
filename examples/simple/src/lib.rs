//! The reference application: one domain-neutral command, end to end.
//!
//! This crate is the worked example M1 asks for, and the shape an application is
//! meant to copy. It depends on `clifacet` the way any application does — through
//! the published API and nothing else — which is what makes it a proof of that API
//! rather than a part of it. It holds the three layers apart on purpose:
//!
//! 1. [`Cli`] is the `clap` surface: flags, help and usage errors. Each argument is
//!    declared once, with `#[clifacet(..)]`: its grammar, its question, its
//!    normalisation and its place on the form sit on the one field (decision-13).
//! 2. [`ExampleInput`] is validated input, built by the [`TryFrom<Cli>`] those
//!    declarations generate, which ends in [`check_input`] for the one rule that is
//!    the application's own. Everything that can be wrong with a value is rejected
//!    there, so [`Input`] is total.
//! 3. [`Example`] is the command: it implements [`Command`], touches no stream and
//!    knows nothing about `clap`.
//!
//! [`run`] is the application layer that joins them: it validates, settles the
//! interaction mode against [`POLICY`], maps failures to the exit codes in [`EXIT`] and
//! writes machine data to one stream and diagnostics to the other.
//! None of that lives in [`Command`], which is what keeps the seam UI-free.
//!
//! # Layout
//!
//! One file per command, and one file per concern of the application around it:
//!
//! ```text
//! src/commands/mod.rs      the registry: one `mod` line per command
//! src/commands/example.rs  the command: `Cli`, `ExampleInput`, `Example`
//! src/interaction.rs       the interaction flags and the mode `POLICY`
//! src/exit.rs              the exit codes, and the one place an outcome is written
//! src/entry.rs             the entry points, and the flag and prompt path
//! src/form.rs              the full-screen form and its submission
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

pub use commands::{Cli, Example, ExampleInput, ExampleOutput, Level, MAX_REPEAT, check_input};
pub use entry::{
    dialog_frontend, main, main_with, main_with_command, run, run_with, run_with_command,
    terminal_frontend,
};
pub use exit::{
    EXIT, EXIT_CANCELLED, EXIT_EXECUTION_FAILED, EXIT_SUCCESS, EXIT_USAGE, EXIT_VALIDATION_FAILED,
    exit_code,
};
pub use form::{run_tui, run_tui_with, tui_form, tui_submit, tui_submit_with};
pub use interaction::{InteractionFlags, POLICY};
