//! `discovered db migrate`: a command inside a group.

/// Apply the pending migrations.
#[clifacet::command(input = Migrated)]
#[derive(Debug, Clone)]
pub struct Migrate {}

/// What `migrate` answers with.
#[derive(Debug, Clone)]
pub struct Migrated {}

impl clifacet::command::Planned for Migrate {
    type Input = Migrated;
    type Output = String;

    fn plan(
        &self,
        _input: Migrated,
    ) -> Result<clifacet::command::Plan<'_, String>, clifacet::Failure> {
        Ok(clifacet::command::Plan::new(|| Ok(String::from("migrated"))).step("apply migrations"))
    }
}
