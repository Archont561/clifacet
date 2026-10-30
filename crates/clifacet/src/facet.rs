//! The companion derive's seam: a clap command that can produce typed input.
//!
//! decision-2 keeps `clap` the grammar authority. This trait therefore does not
//! parse anything itself: it exposes clap's own [`Command`] and bridges an
//! already-parsed struct to the input type a [`Command`](crate::Command) accepts.
//!
//! ```
//! use clifacet::clap::{CommandFactory, Parser};
//! use clifacet::{Facet, Failure};
//!
//! #[derive(clap::Parser, Facet)]
//! #[facet(input = Greeting)]
//! struct Cli {
//!     #[arg(long, alias = "nickname")]
//!     name: Option<String>,
//! }
//!
//! #[derive(Debug, PartialEq)]
//! struct Greeting {
//!     name: Option<String>,
//! }
//!
//! let cli = Cli::try_parse_from(["greet", "--nickname", "ada"]).expect("an alias parses");
//! assert_eq!(cli.validate().expect("valid").name.as_deref(), Some("ada"));
//!
//! // One grammar: the derived command is clap's.
//! assert_eq!(
//!     <Cli as Facet>::command().render_usage().to_string(),
//!     <Cli as CommandFactory>::command().render_usage().to_string(),
//! );
//! ```
//!
//! [`Command`]: clap::Command

use clap::{Command as ClapCommand, CommandFactory};

use crate::failure::Failure;

/// A clap command that bridges to typed, validated input.
///
/// Derive it with `#[derive(clap::Parser, clifacet::Facet)]` and `#[facet(input =
/// Type)]`. The default bridge copies fields by name, so the two structs have to
/// agree on every field — a missing one is a compile error rather than a silent
/// default. When the input normalizes on the way in, declare the bridge explicitly
/// with `#[facet(bridge = path)]` and a `fn(command) -> Result<Input, Failure>`.
///
/// # Unsupported
///
/// - Enums and unions: the bridge is field by field, and a command is a struct.
/// - Tuple and unit structs, for the same reason.
/// - Types that do not implement [`clap::CommandFactory`] — in practice, a struct
///   that does not derive [`clap::Parser`]. A subcommand [`clap::Args`] value has
///   no command of its own to report.
pub trait Facet: CommandFactory {
    /// The validated input this command produces.
    ///
    /// Every value of this type is runnable, which is what lets
    /// [`Command::execute`](crate::Command::execute) reject nothing.
    type Input;

    /// The clap command, borrowed rather than rebuilt.
    ///
    /// This is [`CommandFactory::command`] and nothing else: aliases, defaults,
    /// groups and value parsers stay clap's, so a derived application cannot
    /// drift into a second grammar.
    #[must_use]
    fn command() -> ClapCommand {
        <Self as CommandFactory>::command()
    }

    /// Bridges this parsed command to its typed input.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::validation`] when an application rule rejects the
    /// input. Anything clap rejects never reaches this method: a bad flag, a
    /// failed value parser or a missing required argument is a
    /// [`clap::Error`](clap::Error) at parse time.
    fn validate(self) -> Result<Self::Input, Failure>;
}
