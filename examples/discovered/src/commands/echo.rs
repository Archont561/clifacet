//! `discovered echo`: says back what it was given.

/// Say back what you were given.
#[clifacet::command(input = Echoed)]
#[derive(Debug, Clone)]
pub struct Echo {
    /// The words to repeat.
    #[clifacet(trim, non_empty)]
    pub words: String,
}

/// What `echo` answers with.
#[derive(Debug, Clone)]
pub struct Echoed {
    /// The words, trimmed and never empty.
    pub words: String,
}

impl clifacet::Command for Echo {
    type Input = Echoed;
    type Output = String;

    fn execute(&self, input: Echoed) -> Result<String, clifacet::Failure> {
        Ok(input.words)
    }
}
