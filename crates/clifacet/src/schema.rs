//! Machine-readable CLI schema introspection (CLI-73, decision-25).

use crate::failure::Failure;
use serde::{Deserialize, Serialize};

/// Complete metadata describing a CLI command hierarchy and its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliMetadata {
    /// The command name (e.g. `"fleet"`).
    pub name: String,
    /// Short description from docstrings or about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// Extended description from docstrings or long about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_about: Option<String>,
    /// Package version if declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Package author if declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Arguments, options, and flags accepted by this command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentMetadata>,
    /// Nested subcommands available under this command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<CliMetadata>,
}

/// Metadata describing an argument, option, or flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ArgumentMetadata {
    /// Internal argument identifier (field name or clap id).
    pub id: String,
    /// Long flag name if accepted with `--flag` (without leading hyphens).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
    /// Short flag character if accepted with `-f` (without leading hyphen).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<char>,
    /// Short help text from docstrings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Long help text from docstrings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_help: Option<String>,
    /// Whether this argument must be supplied.
    pub required: bool,
    /// Whether this is a positional argument.
    pub positional: bool,
    /// Whether this is a boolean flag taking no value.
    pub is_flag: bool,
    /// Whether this argument accepts multiple values.
    pub multiple: bool,
    /// Value placeholder name (e.g. `"STRING"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_name: Option<String>,
    /// Default values if not supplied.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_values: Vec<String>,
    /// Allowed possible values (e.g. for enums).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
    /// Environment variable bound to this argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    /// Whether this argument is global across subcommands.
    pub global: bool,
    /// Inferred JSON Schema primitive type (`"string"`, `"boolean"`, `"integer"`, `"array"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
}

impl CliMetadata {
    /// Extracts metadata from a [`clap::Command`] hierarchy.
    #[must_use]
    pub fn from_command(cmd: &clap::Command) -> Self {
        let name = cmd.get_name().to_string();
        let about = cmd.get_about().map(std::string::ToString::to_string);
        let long_about = cmd.get_long_about().map(std::string::ToString::to_string);
        let version = cmd.get_version().map(str::to_string);
        let author = cmd.get_author().map(str::to_string);

        let mut arguments = Vec::new();
        for arg in cmd.get_arguments() {
            let id = arg.get_id().as_str().to_string();
            if id == "help" || id == "version" {
                continue;
            }

            let long = arg.get_long().map(str::to_string);
            let short = arg.get_short();
            let help = arg.get_help().map(std::string::ToString::to_string);
            let long_help = arg.get_long_help().map(std::string::ToString::to_string);
            let required = arg.is_required_set();
            let positional = arg.is_positional();
            let is_flag = matches!(
                arg.get_action(),
                clap::ArgAction::SetTrue | clap::ArgAction::SetFalse
            ) || arg.get_num_args().is_some_and(|r| r.max_values() == 0);
            let multiple = matches!(
                arg.get_action(),
                clap::ArgAction::Append | clap::ArgAction::Count
            );
            let value_name = arg
                .get_value_names()
                .and_then(|v| v.first())
                .map(std::string::ToString::to_string);
            let default_values = arg
                .get_default_values()
                .iter()
                .map(|v| v.to_string_lossy().into_owned())
                .collect();
            let possible_values = arg
                .get_possible_values()
                .into_iter()
                .map(|v| v.get_name().to_string())
                .collect();
            let env = arg.get_env().map(|v| v.to_string_lossy().into_owned());
            let global = arg.is_global_set();

            let value_type = if is_flag {
                Some("boolean".to_string())
            } else if multiple {
                Some("array".to_string())
            } else if let Some(vn) = &value_name {
                let upper = vn.to_uppercase();
                if upper.contains("INT") || upper.contains("NUM") || upper.contains("COUNT") {
                    Some("integer".to_string())
                } else if upper.contains("BOOL") {
                    Some("boolean".to_string())
                } else {
                    Some("string".to_string())
                }
            } else {
                Some("string".to_string())
            };

            arguments.push(ArgumentMetadata {
                id,
                long,
                short,
                help,
                long_help,
                required,
                positional,
                is_flag,
                multiple,
                value_name,
                default_values,
                possible_values,
                env,
                global,
                value_type,
            });
        }

        let mut subcommands = Vec::new();
        for sub in cmd.get_subcommands() {
            let sub_name = sub.get_name();
            if sub_name == "help" {
                continue;
            }
            subcommands.push(Self::from_command(sub));
        }

        Self {
            name,
            about,
            long_about,
            version,
            author,
            arguments,
            subcommands,
        }
    }

    /// Converts this CLI metadata into a valid JSON Schema (Draft 2020-12) representation.
    #[must_use]
    pub fn to_json_schema(&self) -> serde_json::Value {
        command_to_json_schema(self, true)
    }

    /// Serializes the JSON Schema representation as pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if serialization fails.
    pub fn to_json_schema_pretty(&self) -> Result<String, Failure> {
        let val = self.to_json_schema();
        serde_json::to_string_pretty(&val)
            .map_err(|e| Failure::execution(format!("cannot serialize JSON schema: {e}")))
    }
}

impl From<&clap::Command> for CliMetadata {
    fn from(cmd: &clap::Command) -> Self {
        Self::from_command(cmd)
    }
}

/// Serializes `meta` as a pretty-printed JSON Schema string.
///
/// # Errors
///
/// Returns a [`Failure`] if serialization fails.
pub fn to_json_schema_pretty(meta: &CliMetadata) -> Result<String, Failure> {
    meta.to_json_schema_pretty()
}

fn command_to_json_schema(cmd: &CliMetadata, is_root: bool) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    if is_root {
        map.insert(
            "$schema".into(),
            "https://json-schema.org/draft/2020-12/schema".into(),
        );
    }
    map.insert("title".into(), cmd.name.clone().into());
    if let Some(about) = &cmd.about {
        map.insert("description".into(), about.clone().into());
    } else if let Some(long_about) = &cmd.long_about {
        map.insert("description".into(), long_about.clone().into());
    }
    map.insert("type".into(), "object".into());

    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for arg in &cmd.arguments {
        let mut arg_prop = serde_json::Map::new();
        let arg_type = arg.value_type.as_deref().unwrap_or(if arg.is_flag {
            "boolean"
        } else if arg.multiple {
            "array"
        } else {
            "string"
        });

        arg_prop.insert("type".into(), arg_type.into());
        if let Some(help) = &arg.help {
            arg_prop.insert("description".into(), help.clone().into());
        } else if let Some(long_help) = &arg.long_help {
            arg_prop.insert("description".into(), long_help.clone().into());
        }

        if !arg.default_values.is_empty() {
            if arg.multiple {
                arg_prop.insert("default".into(), serde_json::json!(arg.default_values));
            } else if let Some(first) = arg.default_values.first() {
                if arg.is_flag {
                    arg_prop.insert("default".into(), (first == "true").into());
                } else {
                    arg_prop.insert("default".into(), first.clone().into());
                }
            }
        }

        if !arg.possible_values.is_empty() {
            arg_prop.insert("enum".into(), serde_json::json!(arg.possible_values));
        }

        if let Some(env) = &arg.env {
            arg_prop.insert("environment_variable".into(), env.clone().into());
        }

        if let Some(long) = &arg.long {
            arg_prop.insert("flag".into(), format!("--{long}").into());
        }
        if let Some(short) = &arg.short {
            arg_prop.insert("short_flag".into(), format!("-{short}").into());
        }
        arg_prop.insert("positional".into(), arg.positional.into());

        if arg.required {
            required.push(serde_json::Value::String(arg.id.clone()));
        }

        properties.insert(arg.id.clone(), serde_json::Value::Object(arg_prop));
    }

    map.insert("properties".into(), serde_json::Value::Object(properties));
    if !required.is_empty() {
        map.insert("required".into(), serde_json::Value::Array(required));
    }

    if !cmd.subcommands.is_empty() {
        let sub_schemas: Vec<serde_json::Value> = cmd
            .subcommands
            .iter()
            .map(|sub| command_to_json_schema(sub, false))
            .collect();
        map.insert("subcommands".into(), serde_json::Value::Array(sub_schemas));
    }

    serde_json::Value::Object(map)
}
