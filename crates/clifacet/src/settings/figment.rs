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

use std::path::{Path, PathBuf};

use figment::value::{Dict, Map, Value};
use figment::{Figment, Metadata, Profile, Provider};

use super::{Entry, Environment, Error, Loader, Location, Settings, place};

/// A TOML configuration file provider for Figment.
#[derive(Debug, Clone)]
pub struct Toml {
    path: PathBuf,
    optional: bool,
}

impl Toml {
    /// Creates a provider for the TOML file at `path`.
    #[must_use]
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            optional: false,
        }
    }

    /// Marks this provider as optional: if the file does not exist, it is skipped without error.
    #[must_use]
    pub const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

impl Provider for Toml {
    fn metadata(&self) -> Metadata {
        Metadata::from("TOML file", &*self.path)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        if !self.path.is_file() {
            if self.optional {
                return Ok(Map::new());
            }
            return Err(figment::Error::from(format!(
                "{}: configuration file not found",
                self.path.display()
            )));
        }

        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| figment::Error::from(format!("{}: {e}", self.path.display())))?;
        let doc: toml_edit::Document<String> = text
            .parse()
            .map_err(|e| figment::Error::from(format!("{}: {e}", self.path.display())))?;

        let mut map = Map::new();
        let table = doc.as_table();

        // Check if top-level tables define profiles (default, development, staging, production, etc.)
        let known_profiles = [
            "default",
            "global",
            "development",
            "staging",
            "production",
            "test",
            "release",
        ];
        let has_profiles = table
            .iter()
            .any(|(k, item)| item.is_table() && known_profiles.contains(&k));

        if has_profiles {
            for (k, item) in table {
                if let Some(sub_table) = item.as_table() {
                    let profile = if k == "default" || k == "global" {
                        Profile::Default
                    } else {
                        Profile::from(k)
                    };
                    let dict = table_to_dict(sub_table);
                    map.insert(profile, dict);
                }
            }
        } else {
            let dict = table_to_dict(table);
            map.insert(Profile::Default, dict);
        }

        Ok(map)
    }
}

fn table_to_dict(table: &toml_edit::Table) -> Dict {
    let mut dict = Dict::new();
    for (k, item) in table {
        if let Some(val) = item_to_value(item) {
            dict.insert(k.to_string(), val);
        }
    }
    dict
}

fn item_to_value(item: &toml_edit::Item) -> Option<Value> {
    match item {
        toml_edit::Item::Value(val) => Some(value_to_figment(val)),
        toml_edit::Item::Table(table) => Some(Value::from(table_to_dict(table))),
        _ => None,
    }
}

fn value_to_figment(val: &toml_edit::Value) -> Value {
    match val {
        toml_edit::Value::String(s) => Value::from(s.value().as_str()),
        toml_edit::Value::Integer(i) => Value::from(*i.value()),
        toml_edit::Value::Float(f) => Value::from(*f.value()),
        toml_edit::Value::Boolean(b) => Value::from(*b.value()),
        toml_edit::Value::Datetime(d) => Value::from(d.value().to_string()),
        toml_edit::Value::Array(arr) => {
            let vec: Vec<Value> = arr.iter().map(value_to_figment).collect();
            Value::from(vec)
        }
        toml_edit::Value::InlineTable(table) => {
            let mut dict = Dict::new();
            for (k, v) in table {
                dict.insert(k.to_string(), value_to_figment(v));
            }
            Value::from(dict)
        }
    }
}

/// A builder for multi-source cascading Figment configuration.
///
/// Sources are merged in increasing order of priority:
///
/// 1. System configuration (e.g. `/etc/<application>/config.toml`)
/// 2. User configuration (e.g. `~/.config/<application>/config.toml`)
/// 3. Local configuration (e.g. `.<application>.toml` in working directory)
///
/// When an environment variable (`FIGMENT_PROFILE` or `<APP>_PROFILE`) or an
/// explicit profile is specified, that profile is selected on the built [`Figment`].
#[derive(Debug, Clone)]
pub struct Cascade {
    application: String,
    system_path: Option<PathBuf>,
    user_path: Option<PathBuf>,
    local_path: Option<PathBuf>,
    profile: Option<String>,
}

impl Cascade {
    /// Creates a cascading configuration builder for `application`.
    #[must_use]
    pub fn new(application: impl Into<String>) -> Self {
        let app = application.into();
        Self {
            application: app,
            system_path: None,
            user_path: None,
            local_path: None,
            profile: None,
        }
    }

    /// Sets the system configuration file path.
    #[must_use]
    pub fn system(mut self, path: impl Into<PathBuf>) -> Self {
        self.system_path = Some(path.into());
        self
    }

    /// Sets the user configuration file path.
    #[must_use]
    pub fn user(mut self, path: impl Into<PathBuf>) -> Self {
        self.user_path = Some(path.into());
        self
    }

    /// Sets the local configuration file path.
    #[must_use]
    pub fn local(mut self, path: impl Into<PathBuf>) -> Self {
        self.local_path = Some(path.into());
        self
    }

    /// Sets the active profile explicitly.
    #[must_use]
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Builds a [`Figment`] instance layering the cascading sources.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if an existing configuration file contains invalid syntax.
    pub fn build(self, environment: &dyn Environment) -> Result<Figment, Error> {
        let mut figment = Figment::new();

        // 1. System configuration
        let system_file = self.system_path.or_else(|| {
            if let Some(sys_dir) = environment.var("CLIFACET_SYSTEM_DIR") {
                Some(
                    PathBuf::from(sys_dir)
                        .join(&self.application)
                        .join("config.toml"),
                )
            } else {
                #[cfg(target_os = "windows")]
                {
                    environment
                        .var("ProgramData")
                        .map(|p| PathBuf::from(p).join(&self.application).join("config.toml"))
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Some(
                        PathBuf::from("/etc")
                            .join(&self.application)
                            .join("config.toml"),
                    )
                }
            }
        });

        if let Some(path) = system_file {
            figment = figment.merge(Toml::file(path).optional());
        }

        // 2. User configuration
        let user_file = self.user_path.or_else(|| {
            place::config_dir(environment)
                .map(|dir| dir.join(&self.application).join("config.toml"))
        });

        if let Some(path) = user_file {
            figment = figment.merge(Toml::file(path).optional());
        }

        // 3. Local configuration
        let local_file = self.local_path.or_else(|| {
            if let Some(local_dir) = environment.var("CLIFACET_LOCAL_DIR") {
                Some(PathBuf::from(local_dir).join(format!(".{}.toml", self.application)))
            } else {
                Some(PathBuf::from(format!(".{}.toml", self.application)))
            }
        });

        if let Some(path) = local_file {
            figment = figment.merge(Toml::file(path).optional());
        }

        // Profile selection
        let selected_profile = self
            .profile
            .or_else(|| environment.var("FIGMENT_PROFILE"))
            .or_else(|| {
                let var = place::variable(&self.application);
                let prefix = var.strip_suffix("_CONFIG").unwrap_or(&var);
                environment.var(&format!("{prefix}_PROFILE"))
            });

        if let Some(prof) = selected_profile {
            figment = figment.select(prof);
        }

        Ok(figment)
    }

    /// Builds a [`FigmentLoader`] wrapping the cascading Figment configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if an existing configuration file contains invalid syntax.
    pub fn build_loader(self, environment: &dyn Environment) -> Result<FigmentLoader, Error> {
        let figment = self.build(environment)?;
        Ok(FigmentLoader::new(figment))
    }
}

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

    /// Creates a cascading configuration loader for `application` using `environment`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if an existing configuration file contains invalid syntax.
    pub fn cascading(application: &str, environment: &dyn Environment) -> Result<Self, Error> {
        Cascade::new(application).build_loader(environment)
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

        let profile = self.figment.profile();
        let profile_str = profile.as_str().as_str();

        let mut entries = Vec::new();
        flatten(
            &self.figment,
            profile_str,
            &mut Vec::new(),
            &dict,
            &mut entries,
        )?;
        Settings::new(entries)
    }
}

impl Loader for FigmentLoader {
    fn load(&self, _bytes: &[u8]) -> Result<Settings, Error> {
        self.load_settings()
    }
}

/// Finds the 1-based line of `key` inside `section` of `path`.
fn find_key_line(path: &Path, profile: &str, section: &[String], key: &str) -> Option<usize> {
    let text = std::fs::read_to_string(path).ok()?;
    let doc: toml_edit::Document<String> = text.parse().ok()?;

    // 1. If profile is specified, try searching in that profile table first.
    let mut found_item = None;
    if !profile.is_empty()
        && profile != "default"
        && let Some(profile_table) = doc.as_item().get(profile)
    {
        found_item = find_in(profile_table, section, key);
    }

    // 2. Try direct root navigation.
    if found_item.is_none() {
        found_item = find_in(doc.as_item(), section, key);
    }

    // 3. Try under "default" profile table.
    if found_item.is_none()
        && let Some(default_table) = doc.as_item().get("default")
    {
        found_item = find_in(default_table, section, key);
    }

    // 4. Try under any other profile table.
    if found_item.is_none()
        && let Some(table) = doc.as_item().as_table_like()
    {
        for (_, profile_item) in table.iter() {
            if profile_item.is_table()
                && let Some(found) = find_in(profile_item, section, key)
            {
                found_item = Some(found);
                break;
            }
        }
    }

    let item = found_item?;
    let span = item.span()?;
    let line = text
        .as_bytes()
        .iter()
        .take(span.start)
        .filter(|&&b| b == b'\n')
        .count()
        + 1;
    Some(line)
}

fn find_in<'a>(
    root: &'a toml_edit::Item,
    section: &[String],
    key: &str,
) -> Option<&'a toml_edit::Item> {
    let mut current = root;
    for seg in section {
        current = if let Some(item) = current.get(seg) {
            item
        } else {
            let table = current.as_table_like()?;
            let mut found = None;
            for (k, val) in table.iter() {
                if k.replace('-', "_") == seg.replace('-', "_") {
                    found = Some(val);
                    break;
                }
            }
            found?
        };
    }
    if let Some(item) = current.get(key) {
        Some(item)
    } else {
        let table = current.as_table_like()?;
        for (k, val) in table.iter() {
            if k.replace('-', "_") == key.replace('-', "_") {
                return Some(val);
            }
        }
        None
    }
}

fn flatten(
    figment: &Figment,
    profile: &str,
    section: &mut Vec<String>,
    dict: &Dict,
    entries: &mut Vec<Entry>,
) -> Result<(), Error> {
    for (key, value) in dict {
        match value {
            Value::Dict(_, nested) => {
                section.push(key.clone());
                flatten(figment, profile, section, nested, entries)?;
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
                    let mut location = Location::new(path);
                    if let Some(line) = find_key_line(path, profile, section, key) {
                        location = location.at(line);
                    }
                    entry = entry.at(location);
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
