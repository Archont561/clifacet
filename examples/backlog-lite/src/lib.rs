//! A friendly wrapper around the Backlog.md CLI, built on the published API.
//!
//! The point of this crate is not to reimplement Backlog.md. It is to show what a
//! *thin* layer over somebody else's CLI costs when the layer is built on clifacet:
//! typed input ([`Request`]) instead of a stringly argument soup, a decision that is
//! data ([`Invocation`]) instead of a side effect, questions asked only when something
//! can answer them, and a run that can be printed before it happens.
//!
//! ```text
//! bl                          # list, the thing you do most
//! bl --json show CLI-20       # one --json, not one per subcommand
//! bl new "Ship it" --ac one   # the title is optional; you are asked
//! bl done CLI-20              # confirmed, and --yes skips the question
//! bl --dry-run done CLI-20    # prints the exact command instead of running it
//! ```
//!
//! Every leaf builds an argument vector and never touches the process; [`run`] is the
//! one place that spawns anything. That split is what lets the tests assert the
//! command line byte for byte. [`Backlog`] is the wrapper as a clifacet
//! [`Planned`](clifacet::command::Planned) command: `--dry-run` prints its plan and a
//! real run executes the same plan, which the library makes the only execution path —
//! so the preview is honest rather than a separate, slowly-drifting code path.
//!
//! # Layout
//!
//! One file per command, and one file per layer around them:
//!
//! ```text
//! src/commands/mod.rs   the map: the registry, which declares `Cli`, `Commands`, `Request`
//! src/commands/<leaf>.rs  one command: its flags, its input, its questions, its argv
//! src/commands/task.rs  what show, start and done share: a task ID and moving it
//! src/status.rs         the `Status` every spelling of a status parses to
//! src/invocation.rs     a request as a command line, and the one place it runs
//! src/app.rs            modes, `--dry-run`, streams and exit codes
//! ```
//!
//! Everything public is re-exported here, so a caller writes
//! `clifacet_backlog_lite::Cli` and never needs to know which file it lives in.

mod app;
mod commands;
mod invocation;
mod status;

pub use app::{Backlog, EXIT, app, main, request_of, terminal_frontend};
pub use commands::{
    Action, Checkup, Cli, Commands, Filter, Input, NewTask, Query, Request, TaskRef, collect_new,
    confirm_status, task_id,
};
pub use invocation::{BIN_ENV, Invocation, invocation_of, program, run};
pub use status::Status;
