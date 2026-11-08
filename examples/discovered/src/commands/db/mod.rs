//! `discovered db`: a group, which is a folder with its own `clifacet::commands!`.
//!
//! A group is not special: it is a leaf to the folder above it and a folder to the ones
//! below it, exactly as `#[clifacet::subcommands]` has always allowed (decision-14). What
//! discovery adds is that this call can be a bare `commands!()` too, because this folder is
//! reported by the same build script.

/// Work on the database.
#[clifacet::command(input = Chosen)]
#[derive(Clone)]
pub struct Db {
    /// What to do; required here, because a group that did nothing would be a no-op.
    #[clifacet(subcommands)]
    pub command: Commands,
}

/// What `db` was asked to do.
#[derive(Debug, Clone)]
pub struct Chosen {
    /// The chosen subcommand.
    pub command: Request,
}

// This folder's own registration, also discovered: the build script scanned it because it
// is a group of the folder above.
// `planned`: the group's subcommands describe themselves, so the tree previews as well as
// runs — which is what makes `db migrate --dry-run` reach `migrate`'s own plan.
clifacet::commands!(
    output = crate::Said,
    planned
);

impl clifacet::command::Planned for Db {
    type Input = Chosen;
    type Output = crate::Said;

    fn plan(
        &self,
        input: Chosen,
    ) -> Result<clifacet::command::Plan<'_, crate::Said>, clifacet::Failure> {
        clifacet::command::Planned::plan(&self.command, input.command)
    }
}
