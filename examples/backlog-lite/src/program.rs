//! The program the wrapper forwards to: the one thing `bl` depends on.
//!
//! It used to be a variable read from the process environment at the moment an invocation
//! was built. That made a dependency out of process state: a test that wanted a fake had to
//! set a variable every test in the binary would see, and a failure to look it up could
//! only be reported from inside the thing that was supposed to be a pure function.
//!
//! Now it is the application's context (CLI-47), built once per run by the closure the
//! application hands clifacet's runner, and passed to the command: a test injects a fake,
//! and nothing about what runs is read from the environment.

use std::process::Command as Process;

use clifacet::failure::Failure;

/// The command line of the wrapped tool, starting with the program.
///
/// More than one word, because the default in some repositories is
/// `bun x --bun --no-install backlog`. Nothing is shell-expanded: a path containing a space
/// needs a wrapper script rather than quoting, which is the honest cost of not shelling out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    argv: Vec<String>,
}

impl Program {
    /// A program from the words that name it, for example `Program::new(["backlog"])`.
    ///
    /// # Panics
    ///
    /// Panics when the words are empty: a program with no name is not a program, and
    /// catching it here keeps every later call free of the possibility.
    #[must_use]
    pub fn new<I, S>(words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let argv: Vec<String> = words.into_iter().map(Into::into).collect();
        assert!(
            !argv.is_empty(),
            "a program needs at least its own name: `Program::new([\"backlog\"])`"
        );
        Self { argv }
    }

    /// The words, program first.
    #[must_use]
    pub fn argv(&self) -> &[String] {
        &self.argv
    }
}

impl Default for Program {
    /// The tool as a person would have installed it: `backlog` on the `PATH`.
    fn default() -> Self {
        Self::new(["backlog"])
    }
}

/// Runs a built command line, inheriting the terminal, and answers with its exit status.
///
/// The wrapped tool owns its own exit codes and its own output; this returns the child's
/// status unchanged instead of remapping it, because remapping somebody else's contract is
/// how a wrapper starts lying. The wrapper's *own* failures are mapped by the application,
/// which is the one thing clifacet does decide.
///
/// # Errors
///
/// Returns [`Failure::execution`] when the program cannot be spawned at all — a missing
/// `backlog`, usually — and when the child ends without an exit code.
pub fn run(argv: &[String]) -> Result<i32, Failure> {
    let (program, arguments) = argv
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
