//! A request as the command line it stands for.
//!
//! This module is deliberately pure: it turns a validated [`Request`] into an [`Invocation`]
//! and nothing else. The one place that spawns a process is
//! [`program::run`](crate::program::run), and the plan built around it is the application's.

use crate::commands::{self, Action, Filter, Request};
use crate::program::Program;

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
    /// Quoting is deliberately minimal — single quotes only for arguments that contain
    /// whitespace or a quote — so the printed line stays readable.
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

/// Turns a validated request into the command it stands for, on `program`.
///
/// This is a pure function, and that is the design: the same value can be printed
/// (`--dry-run`), asserted in a test, or executed, and the three cannot disagree.
#[must_use]
pub fn invocation_of(request: Request, program: &Program) -> Invocation {
    let mut argv = program.argv().to_vec();
    let json = request.json;
    let action = request.command.unwrap_or(Action::List(Filter::default()));
    let explain = match action {
        Action::List(filter) => commands::forward_list(filter, &mut argv),
        Action::Show(task) => commands::forward_show(task, json, &mut argv),
        Action::New(task) => commands::forward_new(task, &mut argv),
        Action::Start(task) => commands::forward_start(task, &mut argv),
        Action::Done(task) => commands::forward_done(task, &mut argv),
        Action::Search(query) => commands::forward_search(query, &mut argv),
        Action::Doctor(_) => commands::forward_doctor(commands::Checkup {}, &mut argv),
    };
    // One `--json` for the whole run, whatever the leaf forwards: the flag is the root's,
    // so it is appended here rather than repeated per subcommand.
    if json {
        argv.push(String::from("--json"));
    }
    Invocation { argv, explain }
}
