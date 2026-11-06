//! The UI-free command seam.
//!
//! A [`Command`] receives validated input and answers with data or a
//! [`Failure`](crate::Failure). It takes no stream, reads no terminal and holds
//! no presentation state, which is what lets the same command serve flag-based,
//! interactive and TUI frontends (decision-1) without knowing any of them exist.
//!
//! Parsing, validation and rendering stay outside this trait. The application
//! turns its `clap` arguments into `Input`, rejects bad values as
//! [`Failure::validation`](crate::Failure::validation), and writes whatever the
//! command returns. See the `clifacet-example` crate for a complete, tested example of that
//! division.
//!
//! A command that changes something can be [`Planned`] instead: it answers its input
//! with a [`Plan`] — the steps it would take, and the work that takes them — and the
//! library implements [`Command`] for it as "plan, then execute". A `--dry-run` prints
//! the plan's steps; a real run executes the plan; and because the library's
//! `execute` is the only way to run a planned command, the two cannot take different
//! paths (decision-18 item 5).

use std::fmt;

use crate::failure::Failure;

/// A command that runs without a terminal, a prompt or a user interface.
///
/// # Example
///
/// ```
/// use clifacet::{Command, Failure};
///
/// struct Echo;
///
/// impl Command for Echo {
///     type Input = String;
///     type Output = String;
///
///     fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
///         if input.is_empty() {
///             return Err(Failure::execution("nothing to echo"));
///         }
///         Ok(input)
///     }
/// }
///
/// assert_eq!(Echo.execute("hi".to_owned()).unwrap(), "hi");
/// assert_eq!(
///     Echo.execute(String::new()).unwrap_err().kind(),
///     clifacet::FailureKind::Execution,
/// );
/// ```
pub trait Command {
    /// Input the command accepts. Construct it after parsing and validation, so
    /// this type can be total: every value of it is runnable.
    type Input;

    /// Data the application may render or print.
    ///
    /// This is the machine data a caller asked for. It is not a message: the
    /// application decides which stream it belongs on and how it is formatted.
    type Output;

    /// Runs the command against validated input.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::execution`] when the operation cannot complete. Input
    /// problems are rejected before this call, as [`Failure::validation`], by
    /// whoever built the input.
    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure>;
}

/// What a planned command would do, as a value: described first, executed at most once.
///
/// A plan is the steps a person reads — one line each, for a `--dry-run` to print —
/// and the work that performs them. Building one does nothing; [`Plan::execute`] is
/// what runs, and it consumes the plan, so the same plan cannot run twice.
///
/// The steps are written by the command next to the work, from the same input, and
/// the [parity harness](crate::harness) is how a test checks that they describe what
/// the work did: compare a described plan's steps with an executed plan's effects.
///
/// ```
/// use clifacet::command::Plan;
///
/// let plan = Plan::new(|| Ok(2)).step("copy a.txt").step("copy b.txt");
///
/// assert_eq!(plan.describe(), "copy a.txt\ncopy b.txt");
/// assert_eq!(plan.execute().unwrap(), 2);
/// ```
pub struct Plan<'a, T> {
    steps: Vec<String>,
    work: Box<dyn FnOnce() -> Result<T, Failure> + 'a>,
}

impl<'a, T> Plan<'a, T> {
    /// A plan that performs `work`, with no steps described yet.
    pub fn new(work: impl FnOnce() -> Result<T, Failure> + 'a) -> Self {
        Self {
            steps: Vec::new(),
            work: Box::new(work),
        }
    }

    /// Adds a step to the description, after the ones already there.
    #[must_use]
    pub fn step(mut self, step: impl Into<String>) -> Self {
        self.steps.push(step.into());
        self
    }

    /// The steps, in order.
    #[must_use]
    pub fn steps(&self) -> &[String] {
        &self.steps
    }

    /// The steps as text, one per line — what a `--dry-run` prints.
    #[must_use]
    pub fn describe(&self) -> String {
        self.steps.join("\n")
    }

    /// Performs the work.
    ///
    /// # Errors
    ///
    /// Returns whatever [`Failure`] the work reports, as [`Command::execute`] would.
    pub fn execute(self) -> Result<T, Failure> {
        (self.work)()
    }
}

impl<T> fmt::Debug for Plan<'_, T> {
    /// Shows the steps; the work is a closure, and has nothing to show.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Plan")
            .field("steps", &self.steps)
            .finish_non_exhaustive()
    }
}

/// A command that answers its input with a [`Plan`] instead of acting on it.
///
/// Implement this instead of [`Command`]: the library implements [`Command`] for every
/// planned command, and its [`execute`](Command::execute) is `self.plan(input)?.execute()`.
/// That is what makes a dry run trustworthy — there is no second execution path for
/// the plan's description to drift from.
///
/// ```
/// use clifacet::command::{Plan, Planned};
/// use clifacet::{Command, Failure};
///
/// struct Remove;
///
/// impl Planned for Remove {
///     type Input = Vec<String>;
///     type Output = usize;
///
///     fn plan(&self, files: Vec<String>) -> Result<Plan<'_, usize>, Failure> {
///         let steps: Vec<String> = files.iter().map(|file| format!("remove {file}")).collect();
///         let plan = Plan::new(move || Ok(files.len())); // the real removal goes here
///         Ok(steps.into_iter().fold(plan, Plan::step))
///     }
/// }
///
/// let files = vec![String::from("a.txt")];
/// assert_eq!(Remove.plan(files.clone()).unwrap().describe(), "remove a.txt"); // --dry-run
/// assert_eq!(Remove.execute(files).unwrap(), 1); // the real run, through the plan
/// ```
pub trait Planned {
    /// Input the command accepts, validated as for [`Command::Input`].
    type Input;

    /// Data the executed plan produces, as for [`Command::Output`].
    type Output;

    /// What the command would do with `input`, without doing it.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when there is nothing that could be run — the same
    /// failure a real run reports, because a real run is this plan executed.
    fn plan(&self, input: Self::Input) -> Result<Plan<'_, Self::Output>, Failure>;
}

impl<P: Planned> Command for P {
    type Input = P::Input;
    type Output = P::Output;

    /// Plans, then executes the plan: the only way a planned command runs.
    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        self.plan(input)?.execute()
    }
}
