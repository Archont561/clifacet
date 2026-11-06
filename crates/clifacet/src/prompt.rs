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

#[cfg(feature = "interactive")]
mod dialog;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::rc::Rc;

#[cfg(feature = "interactive")]
pub use dialog::Dialog;

use crate::command::Command;
use crate::facet::describe_argument;
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

/// How a confirmation was settled.
///
/// "Not confirmed" has two very different causes — the user said no, or nobody could be
/// asked — and an application usually answers them differently: the first is a decision
/// to cancel, the second is a reason to say "pass `--yes`". Keeping them apart here is
/// what lets a caller stop inspecting the [`Plan`] itself.
///
/// [`Confirmation::is_confirmed`] is the fail-safe boolean for a caller that does not
/// care how: only a yes, or `--yes`, proceeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use = "an unread confirmation is a question nobody acted on"]
pub enum Confirmation {
    /// The user was asked and said yes.
    Confirmed,
    /// The user was asked and said no.
    Declined,
    /// Nobody was asked: `--yes` answered for them.
    Assumed,
    /// Nobody was asked, and nothing could be: the plan asks the user nothing.
    Unavailable,
}

impl Confirmation {
    /// Whether the run may proceed: confirmed by the user, or assumed by `--yes`.
    ///
    /// An unanswerable confirmation is *not* a confirmation, so this is `false` when
    /// nothing could ask — a caller that only reads the boolean fails safe.
    #[must_use]
    pub const fn is_confirmed(self) -> bool {
        matches!(self, Self::Confirmed | Self::Assumed)
    }
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

/// A parsed command that fills in what its flags left out.
///
/// `#[derive(Facet)]` implements it from the field declarations — `ask` for a value no
/// flag supplied, `confirm` for a flag that was set — so a prompted run and a flag run
/// go through the same rule: a supplied value is never re-asked. Collection hands back
/// the same parsed command, now filled in; it is still [`Facet::validate`] that turns it
/// into input, so an answer is checked by exactly the rules a flag is.
///
/// ```
/// use clifacet::policy::{Capabilities, Mode, Policy, Request};
/// use clifacet::prompt::{Answer, Collect, Scripted, Session};
/// use clifacet::Facet;
///
/// #[clifacet::command(input = Greeting)]
/// struct Cli {
///     #[clifacet(long, ask = "Who should I greet?", fill = "world", trim, non_empty)]
///     name: Option<String>,
/// }
///
/// struct Greeting {
///     name: String,
/// }
///
/// let plan = Policy::up_to(Mode::Interactive)
///     .resolve(Request::none(), Capabilities::terminal())
///     .expect("prompts are permitted here");
/// let mut session = Session::start(plan, || {
///     Box::new(Scripted::new([Answer::Text(String::from(" ada "))]))
/// });
///
/// let cli = <Cli as clifacet::clap::Parser>::parse_from(["greet"]);
/// let input = cli.collect(&mut session).and_then(Facet::validate).expect("valid");
/// assert_eq!(input.name, "ada");
/// ```
///
/// [`Facet::validate`]: crate::Facet::validate
pub trait Collect: Sized {
    /// Asks for what is missing and confirms what needs confirming, as the plan permits.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::cancelled`](crate::Failure::cancelled) when the user stops a
    /// question or declines a confirmation, [`Failure::validation`](crate::Failure::validation)
    /// when an answer is not a value the argument's own parser accepts, and the
    /// frontend's failure when it breaks.
    fn collect(self, session: &mut Session) -> Result<Self, Failure>;
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

    /// Confirms, unless `--yes` already answered or nothing may ask, and says which.
    ///
    /// `--yes` settles it as [`Confirmation::Assumed`] without asking; a plan that asks
    /// nothing settles it as [`Confirmation::Unavailable`], which
    /// [`Confirmation::is_confirmed`] reads as *no* — an unanswerable confirmation is
    /// not consent, so a caller that only wants the boolean fails safe.
    ///
    /// # Errors
    ///
    /// Returns the frontend's failure, including cancellation — dismissing the
    /// question is not the same answer as declining it.
    pub fn confirm(&mut self, question: &str) -> Result<Confirmation, Failure> {
        if self.plan.assumes_yes() {
            return Ok(Confirmation::Assumed);
        }
        match self.prompter.as_mut() {
            Some(prompter) => Ok(if prompter.confirm(question)? {
                Confirmation::Confirmed
            } else {
                Confirmation::Declined
            }),
            None => Ok(Confirmation::Unavailable),
        }
    }

    /// Fills an optional argument by asking, when no flag supplied it.
    ///
    /// The answer is parsed by the argument's own `clap` parser — its value parser,
    /// its range, its possible values and its case rule — so an answer and a flag can
    /// never disagree about what is valid, and an inline
    /// `value_parser = clap::value_parser!(u8).range(1..=8)` needs nothing extra.
    /// When nothing may ask, `slot` stays empty and the caller's default decides.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::validation`](crate::Failure::validation) with `clap`'s own
    /// wording when the parser rejects the answer, the frontend's failure (including
    /// cancellation) when the question is not answered, and
    /// [`Failure::execution`](crate::Failure::execution) when `id` names no argument
    /// of `command` that takes a value of type `T` — a declaration mistake, not
    /// something the user did.
    pub fn ask_for<T>(
        &mut self,
        slot: &mut Option<T>,
        command: &clap::Command,
        id: &str,
        question: &str,
    ) -> Result<(), Failure>
    where
        T: Clone + Send + Sync + 'static,
    {
        if slot.is_some() {
            return Ok(());
        }
        let Some(answer) = self.value(None, question)? else {
            return Ok(());
        };
        *slot = Some(parse_answer(command, id, &answer)?);
        Ok(())
    }

    /// Puts a flag that was set to the user before anything runs.
    ///
    /// Asks only when the flag is set and the plan asks the user something: an
    /// unattended `--explode` is a deliberate request, not something to confirm, and
    /// `--yes` answers it without asking. A declined confirmation cancels the run.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::cancelled`](crate::Failure::cancelled), naming the flag,
    /// when the user declines, and the frontend's failure when it breaks.
    pub fn confirm_flag(
        &mut self,
        set: bool,
        command: &clap::Command,
        id: &str,
        question: &str,
    ) -> Result<(), Failure> {
        // Only a refusal stops the run: a flag nobody could be asked about is the
        // unattended request it was typed as.
        if !set || self.confirm(question)? != Confirmation::Declined {
            return Ok(());
        }
        Err(Failure::cancelled(format!(
            "the user declined {question:?} ({})",
            describe_argument(command, id)
        )))
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

/// The subcommand that switches off `clap`'s required-argument pass for an answer.
///
/// An answer is one value, parsed in the command it belongs to so every relation it
/// names still resolves; `subcommand_negates_reqs` is how `clap` itself says "the rest
/// is not being supplied here". The name cannot collide with a user's flag, because
/// a subcommand is only ever matched in a position no answer token reaches.
const ANSWERED: &str = "clifacet-answered";

/// Parses one answer with the argument's own `clap` parser.
///
/// Only public `clap` API is used: the real command is cloned, nothing but the answer
/// may supply a value (environment fallbacks are cleared), and the required-argument
/// pass is negated. `clap` raises a value-parser error while parsing — before any
/// relation is validated — so a rejected answer is reported in `clap`'s words.
fn parse_answer<T>(command: &clap::Command, id: &str, answer: &str) -> Result<T, Failure>
where
    T: Clone + Send + Sync + 'static,
{
    let Some(argument) = command
        .get_arguments()
        .find(|argument| argument.get_id() == id)
    else {
        return Err(Failure::execution(format!(
            "`{id}` is not an argument of `{}`, so its answer cannot be parsed",
            command.get_name()
        )));
    };
    let tokens: Vec<String> = match (argument.get_long(), argument.get_short()) {
        (Some(long), _) => vec![format!("--{long}={answer}")],
        (None, Some(short)) => vec![format!("-{short}"), answer.to_owned()],
        (None, None) => {
            return Err(Failure::execution(format!(
                "`{id}` is positional, and only a named option can be asked for"
            )));
        }
    };

    let probe = command
        .clone()
        .mut_args(|argument| argument.env(None))
        .mut_arg(id, |argument| argument.allow_hyphen_values(true))
        .subcommand(clap::Command::new(ANSWERED))
        .subcommand_negates_reqs(true)
        .subcommand_required(false)
        .arg_required_else_help(false);
    let argv = std::iter::once(command.get_name().to_owned())
        .chain(tokens)
        .chain(std::iter::once(ANSWERED.to_owned()));

    let mut matches = probe
        .try_get_matches_from(argv)
        .map_err(|error| Failure::validation(clap_wording(&error)))?;
    // The answer is always passed as an explicit value, so a parse that succeeded
    // matched the argument; what is left to go wrong is the field and its parser
    // disagreeing about the type. clap's mismatch says how, when it has one.
    match matches.try_remove_one::<T>(id) {
        Ok(Some(value)) => Ok(value),
        other => Err(Failure::execution(format!(
            "the answer for {} parsed to a different type than its field{}",
            describe_argument(command, id),
            other
                .err()
                .map(|mismatch| format!(": {mismatch}"))
                .unwrap_or_default()
        ))),
    }
}

/// `clap`'s message for a rejected value, without the `error:` heading or the help tip.
fn clap_wording(error: &clap::Error) -> String {
    let rendered = error.to_string();
    let message = rendered.strip_prefix("error: ").unwrap_or(&rendered);
    message
        .split("\n\n")
        .next()
        .unwrap_or(message)
        .trim()
        .to_owned()
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
