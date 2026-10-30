//! Which interaction mode a run may use, decided before anything happens.
//!
//! decision-3 fixes the precedence: an **explicit request**, then the
//! **application's policy**, then what the **environment can offer**. This module
//! is that rule and nothing else — no terminal is opened, no prompt is built, no
//! stream is written. Resolution is a pure function of its three inputs, which is
//! what makes it a decision table instead of a guess.
//!
//! ```
//! use clifacet::policy::{Capabilities, Mode, Policy, Request};
//!
//! let policy = Policy::up_to(Mode::Interactive);
//!
//! // An explicit request wins, if the policy permits it.
//! let plan = policy.resolve(Request::mode(Mode::Interactive), Capabilities::terminal())
//!     .expect("prompts are permitted here");
//! assert_eq!(plan.mode(), Mode::Interactive);
//!
//! // Without a request, the policy's preference decides, and capability can only
//! // take interaction away.
//! let plan = policy.resolve(Request::none(), Capabilities::headless())
//!     .expect("a headless run still resolves");
//! assert_eq!(plan.mode(), Mode::NonInteractive);
//!
//! // A terminal on its own is never consent to prompt: the policy says no.
//! assert!(
//!     Policy::non_interactive()
//!         .resolve(Request::mode(Mode::Tui), Capabilities::terminal_with_tui())
//!         .is_err()
//! );
//! ```

use std::io::IsTerminal;

/// How a run talks to the user.
///
/// The ordering is meaningful: interaction only ever *decreases* when a
/// preference is unavailable, so `NonInteractive < Interactive < Tui` is the
/// scale a [`Policy`] ceiling is compared against.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    /// Flags in, data out. Nothing is ever asked.
    #[default]
    NonInteractive,
    /// Prompts on a terminal.
    Interactive,
    /// A full-screen terminal interface.
    Tui,
}

impl Mode {
    /// Whether this mode may ask the user anything.
    ///
    /// This is the single source of truth for "may a frontend be constructed?",
    /// so a forbidden mode cannot reach a prompter by taking a different path.
    #[must_use]
    pub const fn asks_the_user(self) -> bool {
        !matches!(self, Self::NonInteractive)
    }
}

/// What a mode needs that the environment may not provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// A terminal on the input and output side.
    Terminal,
    /// Terminal support for a full-screen interface.
    Tui,
}

/// What the environment can offer a run.
///
/// Construct these explicitly — from a test, from the application, or with
/// [`Capabilities::detected`]. Detection is never treated as consent on its own;
/// see [`Policy`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    terminal: bool,
    tui: bool,
}

impl Capabilities {
    /// No terminal at all: CI, a pipe, a machine consumer.
    #[must_use]
    pub const fn headless() -> Self {
        Self {
            terminal: false,
            tui: false,
        }
    }

    /// A terminal, but no full-screen interface.
    #[must_use]
    pub const fn terminal() -> Self {
        Self {
            terminal: true,
            tui: false,
        }
    }

    /// A terminal that can host a full-screen interface.
    #[must_use]
    pub const fn terminal_with_tui() -> Self {
        Self {
            terminal: true,
            tui: true,
        }
    }

    /// What this process actually has.
    ///
    /// Nothing here claims TUI support: a full-screen frontend arrives later, and
    /// reporting a capability before it exists would let a policy resolve to it.
    #[must_use]
    pub fn detected() -> Self {
        Self {
            terminal: std::io::stdout().is_terminal(),
            tui: false,
        }
    }

    /// Whether these capabilities can host the mode at all.
    #[must_use]
    pub const fn supports(self, mode: Mode) -> bool {
        match mode {
            Mode::NonInteractive => true,
            Mode::Interactive => self.terminal,
            Mode::Tui => self.terminal && self.tui,
        }
    }

    /// What these capabilities are missing for the given mode, if anything.
    ///
    /// Ordered by what a mode needs first: a terminal before TUI support, so the
    /// explanation names the gap the user can act on.
    #[must_use]
    pub const fn missing_for(self, mode: Mode) -> Option<Capability> {
        if self.supports(mode) {
            None
        } else if !self.terminal {
            Some(Capability::Terminal)
        } else {
            Some(Capability::Tui)
        }
    }
}

/// What one invocation asked for, before policy has a say.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Request {
    mode: Option<Mode>,
    assume_yes: bool,
}

impl Request {
    /// The user asked for no particular mode.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            mode: None,
            assume_yes: false,
        }
    }

    /// The user asked for this mode explicitly.
    #[must_use]
    pub const fn mode(mode: Mode) -> Self {
        Self {
            mode: Some(mode),
            assume_yes: false,
        }
    }

    /// Records `--yes`: it answers confirmations, and decides nothing else.
    #[must_use]
    pub const fn assuming_yes(mut self, assume_yes: bool) -> Self {
        self.assume_yes = assume_yes;
        self
    }

    /// The mode the user asked for, if any.
    #[must_use]
    pub const fn requested(self) -> Option<Mode> {
        self.mode
    }

    /// Whether confirmations are answered without asking.
    #[must_use]
    pub const fn assumes_yes(self) -> bool {
        self.assume_yes
    }
}

/// How much interaction an application permits, whatever the environment offers.
///
/// A policy is a ceiling, not a preference list: `Policy::up_to(Mode::Interactive)`
/// means prompts are allowed and a TUI is not. Fewer modes are always allowed,
/// because less interaction never exceeds a permission.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Policy {
    ceiling: Mode,
}

impl Policy {
    /// This application never asks anything.
    #[must_use]
    pub const fn non_interactive() -> Self {
        Self {
            ceiling: Mode::NonInteractive,
        }
    }

    /// The most interactive mode this application is willing to enter.
    #[must_use]
    pub const fn up_to(ceiling: Mode) -> Self {
        Self { ceiling }
    }

    /// The most interactive mode this policy permits.
    #[must_use]
    pub const fn ceiling(self) -> Mode {
        self.ceiling
    }

    /// Applies the precedence and answers with a plan, or with the reason it cannot.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Forbidden`] when the request exceeds this policy's
    /// ceiling, and [`Refusal::Unavailable`] when the environment cannot host the
    /// permitted mode. Neither outcome runs anything.
    pub fn resolve(self, request: Request, capabilities: Capabilities) -> Result<Plan, Refusal> {
        let mode = match request.requested() {
            Some(requested) => {
                if requested > self.ceiling {
                    return Err(Refusal::Forbidden {
                        requested,
                        ceiling: self.ceiling,
                    });
                }
                if let Some(missing) = capabilities.missing_for(requested) {
                    return Err(Refusal::Unavailable { requested, missing });
                }
                requested
            }
            // No request: the most interactive mode this policy permits that the
            // environment supports. Interaction only ever decreases, so a terminal
            // can never add consent the policy withheld.
            None => [Mode::Tui, Mode::Interactive, Mode::NonInteractive]
                .into_iter()
                .find(|candidate| *candidate <= self.ceiling && capabilities.supports(*candidate))
                .unwrap_or(Mode::NonInteractive),
        };

        Ok(Plan {
            mode,
            assume_yes: request.assumes_yes(),
        })
    }
}

/// A permitted way to run: the mode, and whether confirmations are answered.
///
/// Only [`Policy::resolve`] produces one, so "may this run ask the user anything?"
/// has exactly one answer, and it comes from the same place every time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    mode: Mode,
    assume_yes: bool,
}

impl Plan {
    /// The mode this run uses.
    #[must_use]
    pub const fn mode(self) -> Mode {
        self.mode
    }

    /// Whether a frontend may ask the user anything.
    #[must_use]
    pub const fn asks_the_user(self) -> bool {
        self.mode.asks_the_user()
    }

    /// Whether confirmations are answered with yes instead of asked.
    #[must_use]
    pub const fn assumes_yes(self) -> bool {
        self.assume_yes
    }

    /// Builds a frontend, or nothing at all when this plan asks nothing.
    ///
    /// This is the seam a presentation layer goes through. Because the closure is
    /// the only place a prompter or terminal frontend can be constructed, a
    /// non-interactive run cannot build one by accident — there is no other door.
    pub fn with_frontend<T>(self, build: impl FnOnce() -> T) -> Option<T> {
        self.asks_the_user().then(build)
    }
}

/// Why a request could not be honored.
///
/// A refusal is not a [`Failure`]: nothing was asked of the command, and the
/// application decides what the explanation costs. It is also not a
/// cancellation — a cancel is a decision by the user, a refusal is one by the
/// policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// The application's policy does not permit this mode.
    Forbidden {
        /// What the user asked for.
        requested: Mode,
        /// The most interactive mode the policy permits.
        ceiling: Mode,
    },
    /// The environment cannot host the mode.
    Unavailable {
        /// What the user asked for.
        requested: Mode,
        /// What the environment is missing.
        missing: Capability,
    },
}

impl Refusal {
    /// The caller-facing explanation, for a diagnostic stream.
    #[must_use]
    pub fn message(self) -> String {
        match self {
            Self::Forbidden { requested, ceiling } => format!(
                "{} is not available: this command allows at most {}",
                describe(requested),
                describe(ceiling)
            ),
            Self::Unavailable { requested, missing } => format!(
                "{} is not available here: no {}",
                describe(requested),
                match missing {
                    Capability::Terminal => "terminal",
                    Capability::Tui => "TUI support",
                }
            ),
        }
    }
}

/// How a mode is named in a diagnostic.
const fn describe(mode: Mode) -> &'static str {
    match mode {
        Mode::NonInteractive => "non-interactive",
        Mode::Interactive => "interactive use",
        Mode::Tui => "a TUI",
    }
}
