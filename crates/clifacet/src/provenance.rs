//! Where each input value came from, recorded without recording the value.
//!
//! An application that accepts a value from a flag, an environment variable, a
//! default, a prompt or a full-screen form knows which of those it used. [`Provenance`]
//! is where it says so, and it is deliberately unable to say what the value was: an
//! [`Origin`] holds a name, a [`Source`] and whether the value is secret, and nothing
//! else. "The record cannot leak a secret" is therefore a property of the type rather
//! than a rule someone has to remember.
//!
//! ```
//! use clifacet::provenance::{Provenance, Source};
//!
//! let mut provenance = Provenance::default();
//! provenance.record("name", Source::Prompt);
//! provenance.record_secret("token", Source::Environment);
//!
//! assert_eq!(provenance.source_of("token"), Some(Source::Environment));
//! assert_eq!(
//!     provenance.to_string(),
//!     "name <- prompt\ntoken <- environment (secret)",
//! );
//! ```
//!
//! Provenance is not a log. It is the answer to "which frontend supplied this?" — the
//! question a parity test asks after it has already asserted that the answer itself
//! was the same everywhere.

use std::fmt;

/// Where a value came from.
///
/// The variants are the sources the framework can name without guessing. An
/// application reading a config file of its own has no variant here on purpose: adding
/// one for every possible origin would be a registry, and the interesting distinction
/// is whether a person was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// A command-line flag or argument.
    Argv,
    /// An environment variable.
    Environment,
    /// A default the application chose.
    Default,
    /// A question answered through [`crate::prompt`].
    Prompt,
    /// A field of the [`crate::tui`] form.
    Form,
}

impl Source {
    /// How the source is named in a record.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Argv => "argv",
            Self::Environment => "environment",
            Self::Default => "default",
            Self::Prompt => "prompt",
            Self::Form => "form",
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One recorded input: what it was called, where it came from, and whether it is
/// secret.
///
/// The value is absent by design — there is no field to put it in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    name: String,
    source: Source,
    secret: bool,
}

impl Origin {
    /// The input's name, as the application calls it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the value came from.
    #[must_use]
    pub const fn source(&self) -> Source {
        self.source
    }

    /// Whether the value is secret, and so was never recorded.
    #[must_use]
    pub const fn is_secret(&self) -> bool {
        self.secret
    }
}

impl fmt::Display for Origin {
    /// Renders as `name <- source`, with `(secret)` when the value is one.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} <- {}", self.name, self.source)?;
        if self.secret {
            formatter.write_str(" (secret)")?;
        }
        Ok(())
    }
}

/// The record of where each input value came from.
///
/// Recording is explicit because only the application knows: the framework can see
/// that a value was supplied rather than asked, not whether it arrived on the command
/// line or from the environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provenance {
    origins: Vec<Origin>,
}

impl Provenance {
    /// Records an input and its source, replacing an earlier record for the same name.
    pub fn record(&mut self, name: &str, source: Source) -> &mut Self {
        self.insert(name, source, false)
    }

    /// Records an input whose value must never be shown.
    pub fn record_secret(&mut self, name: &str, source: Source) -> &mut Self {
        self.insert(name, source, true)
    }

    /// Records an input, keeping one entry per name so a re-ask does not duplicate it.
    fn insert(&mut self, name: &str, source: Source, secret: bool) -> &mut Self {
        let origin = Origin {
            name: name.to_owned(),
            source,
            secret,
        };

        match self.origins.iter_mut().find(|entry| entry.name == name) {
            Some(entry) => *entry = origin,
            None => self.origins.push(origin),
        }
        self
    }

    /// Every recorded input, in the order the names first appeared.
    #[must_use]
    pub fn entries(&self) -> &[Origin] {
        &self.origins
    }

    /// Where the named input came from, if it was recorded.
    #[must_use]
    pub fn source_of(&self, name: &str) -> Option<Source> {
        self.origins
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.source)
    }

    /// Whether the named input was recorded as secret.
    #[must_use]
    pub fn is_secret(&self, name: &str) -> bool {
        self.origins
            .iter()
            .any(|entry| entry.name == name && entry.secret)
    }

    /// How many inputs were recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.origins.len()
    }

    /// Whether nothing was recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.origins.is_empty()
    }
}

impl fmt::Display for Provenance {
    /// One line per input, in the order they were recorded.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, origin) in self.origins.iter().enumerate() {
            if index > 0 {
                formatter.write_str("\n")?;
            }
            write!(formatter, "{origin}")?;
        }
        Ok(())
    }
}
