//! Reading settings through [`figment::Figment`].
//!
//! A table is a section and a key is a flag name:
//!
//! ```ignore
//! [serve]
//! port = 8080
//! host = "0.0.0.0"
//!
//! [serve.retry]
//! attempts = 3
//! ```
//!
//! supplies `--port` and `--host` to the `serve` subcommand, and `--attempts` to
//! `serve retry`. `kebab-case` and `snake_case` keys are reconciled automatically, so
//! `listen_port` in configuration matches `--listen-port` on the command line.
//!
//! Values extracted from Figment files retain their provenance, recording
//! [`Source::File`](crate::provenance::Source::File) when injected into `clap`.

use figment::Figment;
use figment::value::{Dict, Value};

use super::{Entry, Error, Loader, Location, Settings};

/// A loader that extracts [`Settings`] from a [`figment::Figment`].
#[derive(Debug, Clone)]
pub struct FigmentLoader {
    figment: Figment,
}

impl FigmentLoader {
    /// Creates a loader wrapping `figment`.
    #[must_use]
    pub fn new(figment: Figment) -> Self {
        Self { figment }
    }

    /// The underlying [`Figment`].
    #[must_use]
    pub fn figment(&self) -> &Figment {
        &self.figment
    }

    /// Extracts [`Settings`] from the wrapped Figment instance.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if extracting dictionary values fails or if duplicate keys are found.
    pub fn load_settings(&self) -> Result<Settings, Error> {
        let dict: Dict = self
            .figment
            .extract()
            .map_err(|error| Error::new(error.to_string()))?;

        let mut entries = Vec::new();
        flatten(&self.figment, &mut Vec::new(), &dict, &mut entries)?;
        Settings::new(entries)
    }
}

impl Loader for FigmentLoader {
    fn load(&self, _bytes: &[u8]) -> Result<Settings, Error> {
        self.load_settings()
    }
}

fn flatten(
    figment: &Figment,
    section: &mut Vec<String>,
    dict: &Dict,
    entries: &mut Vec<Entry>,
) -> Result<(), Error> {
    for (key, value) in dict {
        match value {
            Value::Dict(_, nested) => {
                section.push(key.clone());
                flatten(figment, section, nested, entries)?;
                section.pop();
            }
            Value::Empty(..) => {}
            Value::Array(..) => {
                let dotted = if section.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", section.join("."), key)
                };
                return Err(Error::new(format!(
                    "`{dotted}` holds several values, and a setting is one"
                )));
            }
            scalar => {
                let text = match scalar {
                    Value::String(_, s) => s.clone(),
                    Value::Char(_, c) => c.to_string(),
                    Value::Bool(_, b) => b.to_string(),
                    Value::Num(_, n) => num_to_string(n),
                    _ => unreachable!(),
                };
                let dotted = if section.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", section.join("."), key)
                };
                let section_refs: Vec<&str> = section.iter().map(String::as_str).collect();
                let mut entry = Entry::new(&section_refs, key.clone(), text);
                if let Some(metadata) = figment.find_metadata(&dotted)
                    && let Some(figment::Source::File(path)) = &metadata.source
                {
                    entry = entry.at(Location::new(path));
                }
                entries.push(entry);
            }
        }
    }
    Ok(())
}

fn num_to_string(n: &figment::value::Num) -> String {
    use figment::value::Num;
    match n {
        Num::U8(v) => v.to_string(),
        Num::U16(v) => v.to_string(),
        Num::U32(v) => v.to_string(),
        Num::U64(v) => v.to_string(),
        Num::U128(v) => v.to_string(),
        Num::USize(v) => v.to_string(),
        Num::I8(v) => v.to_string(),
        Num::I16(v) => v.to_string(),
        Num::I32(v) => v.to_string(),
        Num::I64(v) => v.to_string(),
        Num::I128(v) => v.to_string(),
        Num::ISize(v) => v.to_string(),
        Num::F32(v) => v.to_string(),
        Num::F64(v) => v.to_string(),
    }
}
