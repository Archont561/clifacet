//! Collecting what the flags left out, without asking anything that was supplied.
//!
//! decision-3 splits the mode decision ([`crate::policy`]) from the frontend. This
//! module is the frontend side: a [`Prompter`] is *somewhere to ask a question*,
//! and a [`Session`] is the rule that decides whether a question may be asked at
//! all.
//!
//! The rule is one sentence long: **a supplied value is never re-asked, a missing
//! one is asked for only when the plan permits it.** That is what makes a prompted
//! run and a flag run the same command rather than two behaviours — parity is a
//! property of the session, not a promise in a document.
//!
//! ```
//! use clifacet::policy::{Capabilities, Policy};
//! use clifacet::prompt::{Answer, Scripted, Session};
//!
//! let plan = Policy::non_interactive()
//!     .resolve(clifacet::policy::Request::none(), Capabilities::headless())
//!     .expect("the default resolves");
//! let mut session = Session::start(plan, || Box::new(Scripted::default()));
//!
//! // Nothing can ask here, so a missing value stays missing instead of blocking.
//! assert_eq!(session.value(None, "Who should I greet?").expect("no question"), None);
//! // A supplied value is taken as given, never re-asked.
//! assert_eq!(
//!     session.value(Some(String::from("ada")), "Who?").expect("no question").as_deref(),
//!     Some("ada"),
//! );
//! ```
//!
//! The two adapters here are the ones 0.1 needs: [`Terminal`] for a person, and
//! [`Scripted`] for tests and for scripted flows. Neither knows what it is asking
//! about, which is why both fit any command.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::rc::Rc;

use crate::command::Command;
use crate::failure::Failure;
use crate::policy::Plan;

/// Somewhere to ask the user a question.
///
/// Implementations decide how the question is shown and how the answer is read.
/// Cancellation is not an error condition to be papered over: a frontend that is
/// dismissed, or whose input ends, returns [`Failure::cancelled`] so the
/// application can report it as its own category and run nothing.
///
/// [`Failure::cancelled`]: crate::Failure::cancelled
pub trait Prompter {
    /// Asks for a value.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::cancelled`](crate::Failure::cancelled) when the user
    /// stops the question, and a failure of any kind when the frontend itself
    /// breaks.
    fn ask(&mut self, question: &str) -> Result<String, Failure>;

    /// Asks a yes/no question.
    ///
    /// # Errors
    ///
    /// As [`Prompter::ask`].
    fn confirm(&mut self, question: &str) -> Result<bool, Failure>;
}

/// What a scripted frontend answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Some typed text.
    Text(String),
    /// Confirmed.
    Yes,
    /// Declined.
    No,
    /// Dismissed: the user stopped the flow.
    Cancel,
}

/// A frontend that answers from a fixed script.
///
/// This is the framework-owned seam decision-6 and CLI-10 ask for: a flow can be
/// exercised end to end without a terminal, and it cannot accidentally read one.
/// Clones share their state, so a test keeps a handle to read back what was asked.
#[derive(Debug, Clone, Default)]
pub struct Scripted {
    state: Rc<RefCell<Script>>,
}

#[derive(Debug, Default)]
struct Script {
    answers: VecDeque<Answer>,
    asked: Vec<String>,
}

impl Scripted {
    /// Answers the questions in order.
    #[must_use]
    pub fn new(answers: impl IntoIterator<Item = Answer>) -> Self {
        Self {
            state: Rc::new(RefCell::new(Script {
                answers: answers.into_iter().collect(),
                asked: Vec::new(),
            })),
        }
    }

    /// The questions asked so far, in order.
    #[must_use]
    pub fn asked(&self) -> Vec<String> {
        self.state.borrow().asked.clone()
    }

    /// The answers the script has left.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.state.borrow().answers.len()
    }

    /// Takes the next answer, recording the question it belongs to.
    fn next(&self, question: &str) -> Result<Answer, Failure> {
        let mut state = self.state.borrow_mut();
        state.asked.push(question.to_owned());
        state.answers.pop_front().ok_or_else(|| {
            Failure::execution(format!(
                "the scripted prompter has no answer for {question:?}: add one to the script"
            ))
        })
    }
}

impl Prompter for Scripted {
    fn ask(&mut self, question: &str) -> Result<String, Failure> {
        match self.next(question)? {
            Answer::Text(text) => Ok(text),
            Answer::Cancel => Err(Failure::cancelled(format!(
                "the user dismissed {question:?}"
            ))),
            // A script that says "yes" where text was needed is a broken script,
            // not a name: failing here is better than greeting someone "yes".
            answer => Err(Failure::execution(format!(
                "the script answered the question {question:?} with {answer:?}, \
                 which is not text"
            ))),
        }
    }

    fn confirm(&mut self, question: &str) -> Result<bool, Failure> {
        match self.next(question)? {
            Answer::Yes => Ok(true),
            Answer::No => Ok(false),
            Answer::Cancel => Err(Failure::cancelled(format!(
                "the user dismissed {question:?}"
            ))),
            Answer::Text(text) => Err(Failure::execution(format!(
                "the script answered {question:?} with text where yes or no was needed: {text:?}"
            ))),
        }
    }
}

/// A frontend that asks a person on a terminal.
///
/// It is generic over its streams so the same code can be driven by a cursor in a
/// test and by the process in production. Questions go to the *diagnostic* stream:
/// the data stream stays machine-readable, which is the same rule `Command`
/// follows for output.
#[derive(Debug)]
pub struct Terminal<R, W> {
    input: R,
    prompt: W,
}

impl<R: BufRead, W: Write> Terminal<R, W> {
    /// Reads answers from `input` and writes questions to `prompt`.
    pub fn new(input: R, prompt: W) -> Self {
        Self { input, prompt }
    }

    /// Asks one question and reads one line.
    fn line(&mut self, question: &str) -> Result<String, Failure> {
        write!(self.prompt, "{question} ").map_err(|error| write_failed(&error))?;
        self.prompt.flush().map_err(|error| write_failed(&error))?;

        let mut answer = String::new();
        let read = self
            .input
            .read_line(&mut answer)
            .map_err(|error| read_failed(&error))?;
        if read == 0 {
            // End of input: nobody is there to answer, which is a cancellation and
            // not an empty answer — a blank value would silently become a default.
            return Err(Failure::cancelled(format!(
                "the input ended while asking {question:?}"
            )));
        }
        Ok(answer.trim().to_owned())
    }
}

impl Terminal<std::io::StdinLock<'static>, std::io::Stderr> {
    /// Asks the process's own terminal: answers from stdin, questions on stderr.
    #[must_use]
    pub fn stdio() -> Self {
        Self::new(std::io::stdin().lock(), std::io::stderr())
    }
}

impl<R: BufRead, W: Write> Prompter for Terminal<R, W> {
    fn ask(&mut self, question: &str) -> Result<String, Failure> {
        self.line(question)
    }

    fn confirm(&mut self, question: &str) -> Result<bool, Failure> {
        loop {
            match self.line(question)?.to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                // Re-ask rather than guess: a confirmation that defaults on a typo
                // is worse than one more question.
                _ => (),
            }
        }
    }
}

fn write_failed(error: &std::io::Error) -> Failure {
    Failure::execution(format!("cannot write the prompt: {error}"))
}

fn read_failed(error: &std::io::Error) -> Failure {
    Failure::execution(format!("cannot read the answer: {error}"))
}

/// One run's questions, and the rule for whether they may be asked.
///
/// A session is built from a [`Plan`], so "may we prompt?" has already been
/// answered before the first question: a plan that asks nothing produces a session
/// with no frontend, and the frontend builder is never called.
pub struct Session {
    plan: Plan,
    prompter: Option<Box<dyn Prompter>>,
}

impl Session {
    /// Prepares the questions for this plan.
    ///
    /// `build` is called only when the plan asks the user something, so a
    /// non-interactive run never constructs a frontend at all.
    pub fn start(plan: Plan, build: impl FnOnce() -> Box<dyn Prompter>) -> Self {
        Self {
            plan,
            prompter: plan.with_frontend(build),
        }
    }

    /// The plan this session collects against.
    #[must_use]
    pub const fn plan(&self) -> Plan {
        self.plan
    }

    /// Takes the supplied value, or asks for one when the plan permits.
    ///
    /// Returns `None` when the value is missing and nothing may ask, leaving the
    /// caller's own default in charge. A supplied value is returned untouched —
    /// that is the parity guarantee: prompting never changes an explicit choice.
    ///
    /// # Errors
    ///
    /// Returns the frontend's failure, including
    /// [`Failure::cancelled`](crate::Failure::cancelled) when the user stops.
    pub fn value(
        &mut self,
        supplied: Option<String>,
        question: &str,
    ) -> Result<Option<String>, Failure> {
        if supplied.is_some() {
            return Ok(supplied);
        }
        match self.prompter.as_mut() {
            Some(prompter) => prompter.ask(question).map(Some),
            None => Ok(None),
        }
    }

    /// Confirms, unless `--yes` already answered or nothing may ask.
    ///
    /// When the plan asks nothing this returns `Ok(false)`: an unanswerable
    /// confirmation is *not* a confirmation, so a caller that forgets to check the
    /// plan fails safe instead of proceeding.
    ///
    /// # Errors
    ///
    /// Returns the frontend's failure, including cancellation.
    pub fn confirm(&mut self, question: &str) -> Result<bool, Failure> {
        if self.plan.assumes_yes() {
            return Ok(true);
        }
        match self.prompter.as_mut() {
            Some(prompter) => prompter.confirm(question),
            None => Ok(false),
        }
    }

    /// Collects the input, then executes — never the other way round.
    ///
    /// Keeping the two in one place is what makes "nothing runs unless collection
    /// succeeded" structural: a cancelled question, a bad answer or an exhausted
    /// script all return before the command is reachable.
    ///
    /// # Errors
    ///
    /// Returns whatever collection or the command reports.
    pub fn run<C: Command>(
        &mut self,
        command: &C,
        collect: impl FnOnce(&mut Self) -> Result<C::Input, Failure>,
    ) -> Result<C::Output, Failure> {
        let input = collect(self)?;
        command.execute(input)
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("plan", &self.plan)
            .field("prompter", &self.prompter.as_ref().map(|_| "constructed"))
            .finish()
    }
}
