//! How interactive a run may be, and what the user asked for.

use clifacet::Globals;
use clifacet::policy::{Mode, Policy, Request};

use crate::commands::Example;

/// The interaction flags, kept together because they are one decision.
///
/// `--interactive` and `--tui` are this application's; `--yes` and `--dry-run` are the
/// ones every clifacet application gets, from [`Globals`].
#[derive(Debug, Clone, Default, PartialEq, Eq, clap::Args)]
pub struct InteractionFlags {
    /// Ask for prompts rather than flags.
    #[arg(long, conflicts_with = "tui")]
    pub interactive: bool,

    /// Ask for a full-screen interface.
    ///
    /// Always honoured by this crate, which enables the `tui` frontend; an
    /// application that does not would have it refused with a message, because a
    /// mode nothing can draw is not a mode.
    #[arg(long)]
    pub tui: bool,

    /// `--yes`, which answers confirmations and decides nothing else, and `--dry-run`,
    /// which this command refuses because it cannot show what it would do.
    #[command(flatten)]
    pub globals: Globals,
}

/// How interactive this application is, in both senses.
///
/// **Permission**: prompts and the full-screen form are both allowed — [`app`] asks
/// with the dialog for the first and the runner hosts the second. The
/// crate enables both frontends, so the ceiling is the most interactive mode the
/// library has: an application that enables only prompts would pass
/// `Mode::Interactive` here and have `--tui` refused instead.
///
/// **Intent**: prompts are the default. A run with no flags asks for what the flags
/// left out, because that is the mode a person at a terminal expects; the form is
/// what `--tui` asks for, not what a bare invocation gets. Stating the preference is
/// all this line adds — the ceiling still has the final say on what is permitted, and
/// the environment on what is possible.
///
/// [`app`]: crate::app
pub const POLICY: Policy = Policy::up_to(Mode::Tui).prefer(Mode::Interactive);

/// The mode the user asked for, if any.
///
/// `clap` rejects `--interactive --tui` before this runs, so at most one of them
/// is set and the order here never decides anything. `--yes` is not read here: the
/// runner adds it to whatever mode this returns.
pub(crate) fn requested_mode(cli: &Example) -> Request {
    match (cli.interaction.tui, cli.interaction.interactive) {
        (true, _) => Request::mode(Mode::Tui),
        (false, true) => Request::mode(Mode::Interactive),
        (false, false) => Request::none(),
    }
}
