//! `discovered list`: says what this folder registered.

/// List the commands discovery found.
#[clifacet::command(input = Listed)]
#[derive(Debug, Clone)]
pub struct List {}

/// What `list` answers with: the greeting, from this folder's helper.
#[derive(Debug, Clone)]
pub struct Listed {}

impl clifacet::Command for List {
    type Input = Listed;
    type Output = String;

    fn execute(&self, _input: Listed) -> Result<String, clifacet::Failure> {
        Ok(format!("{}: list, echo, db", super::_shared::greeting()))
    }
}
