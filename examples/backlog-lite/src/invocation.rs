//! A validated request as the command line it stands for, and the one place it runs.
//!
//! Each leaf forwards its own arguments (`forward` in its file); this module only
//! picks the leaf, adds what every leaf shares — the program and `--json` — and is the
//! single place that spawns a process.

use std::process::Command as Process;

use clifacet::failure::Failure;

use crate::commands::{self, Action, Filter, Request};

/// The program the wrapper runs, overridable so a test never spawns the real tool.
///
/// The value is a command with whitespace-separated arguments — `backlog` by
/// default, `bun x --bun --no-install backlog` in a repository that runs the CLI
/// through Bun. Nothing is shell-expanded, so a path containing a space needs a
/// wrapper script rather than quoting; the honest cost of not shelling out.
pub const BIN_ENV: &str = "CLIFACET_BACKLOG_BIN";

/// The command line this request stands for, plus why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    argv: Vec<String>,
    explain: String,
}

impl Invocation {
    /// The argument vector, starting with the program.
    #[must_use]
    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    /// One sentence saying what the command does, for `--dry-run` to print.
    #[must_use]
    pub fn explain(&self) -> &str {
        &self.explain
    }

    /// The command as a line that can be pasted into a shell.
    ///
    /// Quoting is deliberately minimal — single quotes only for arguments that
    /// contain whitespace or a quote — so the printed line stays readable.
    #[must_use]
    pub fn shell_line(&self) -> String {
        self.argv
            .iter()
            .map(|argument| {
                if argument.is_empty()
                    || argument
                        .chars()
                        .any(|character| character.is_whitespace() || character == '\'')
                {
                    format!("'{}'", argument.replace('\'', r"'\''"))
                } else {
                    argument.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Appends fixed arguments, for the leaves' `forward` functions.
pub(crate) fn push_args(argv: &mut Vec<String>, values: &[&str]) {
    argv.extend(values.iter().map(|value| (*value).to_owned()));
}

/// Turns a validated request into the command it stands for.
///
/// This is a pure function, and that is the design: the same value can be printed
/// (`--dry-run`), asserted in a test, or executed, and the three cannot disagree.
#[must_use]
pub fn invocation_of(request: Request) -> Invocation {
    let mut argv = program();
    let action = request
        .command
        .unwrap_or_else(|| Action::List(Filter::default()));
    let explain = match action {
        Action::List(filter) => commands::forward_list(filter, &mut argv),
        Action::Show(task) => commands::forward_show(task, request.json, &mut argv),
        Action::New(task) => commands::forward_new(task, &mut argv),
        Action::Start(task) => commands::forward_start(task, &mut argv),
        Action::Done(task) => commands::forward_done(task, &mut argv),
        Action::Search(query) => commands::forward_search(query, &mut argv),
        Action::Doctor(checkup) => commands::forward_doctor(checkup, &mut argv),
    };
    if request.json {
        argv.push(String::from("--json"));
    }
    Invocation { argv, explain }
}

/// The command to run: the environment's override, or `backlog`.
#[must_use]
pub fn program() -> Vec<String> {
    std::env::var(BIN_ENV).map_or_else(
        |_| vec![String::from("backlog")],
        |value| {
            let parts: Vec<String> = value.split_whitespace().map(str::to_owned).collect();
            if parts.is_empty() {
                vec![String::from("backlog")]
            } else {
                parts
            }
        },
    )
}

/// Runs an invocation, inheriting the terminal, and answers with its exit status.
///
/// The wrapped tool owns its own exit codes, and its own output; this returns the
/// child's status unchanged instead of remapping it, because remapping somebody
/// else's contract is how a wrapper starts lying. The wrapper's *own* failures are
/// mapped by the application, which is the one thing clifacet does decide.
///
/// # Errors
///
/// Returns [`Failure::execution`] when the program cannot be spawned at all — a
/// missing `backlog`, usually — and when the child ends without an exit code.
pub fn run(invocation: &Invocation) -> Result<i32, Failure> {
    let (program, arguments) = invocation
        .argv()
        .split_first()
        .ok_or_else(|| Failure::execution("nothing to run"))?;
    let status = Process::new(program)
        .args(arguments)
        .status()
        .map_err(|error| Failure::execution(format!("cannot run {program}: {error}")))?;
    status
        .code()
        .ok_or_else(|| Failure::execution(format!("{program} ended without an exit code")))
}
