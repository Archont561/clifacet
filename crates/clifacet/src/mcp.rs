//! Model Context Protocol (MCP) stdio tool server implementation (decision-25, CLI-77).
//!
//! Exposes an application's controllers and commands as tools to AI agent hosts
//! (such as Claude Desktop, Cursor, or any MCP-compatible client) over a standard
//! input/output JSON-RPC stream.

use std::io::{BufRead, Write};

use crate::failure::Failure;
use crate::schema::{ArgumentMetadata, CliMetadata};

/// Model Context Protocol (MCP) tool server running over JSON-RPC 2.0.
pub struct McpServer<'a, F> {
    meta: &'a CliMetadata,
    runner: F,
}

impl<'a, F> McpServer<'a, F>
where
    F: Fn(&str, &serde_json::Value) -> Result<String, Failure>,
{
    /// Creates a new MCP server for the given CLI metadata and command runner.
    #[must_use]
    pub fn new(meta: &'a CliMetadata, runner: F) -> Self {
        Self { meta, runner }
    }

    /// Handles a parsed JSON-RPC request and returns the response if one should be sent.
    ///
    /// Notifications (such as `notifications/initialized`) return `None`.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn handle_request(&self, request: &serde_json::Value) -> Option<serde_json::Value> {
        let id = request.get("id").cloned();
        let jsonrpc = request.get("jsonrpc").and_then(serde_json::Value::as_str);

        if jsonrpc != Some("2.0") {
            return Some(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id.unwrap_or(serde_json::Value::Null),
                "error": {
                    "code": -32600,
                    "message": "Invalid Request: missing or invalid jsonrpc version"
                }
            }));
        }

        let Some(method) = request.get("method").and_then(serde_json::Value::as_str) else {
            return Some(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id.unwrap_or(serde_json::Value::Null),
                "error": {
                    "code": -32600,
                    "message": "Invalid Request: missing method"
                }
            }));
        };

        match method {
            "initialize" => {
                let req_id = id.unwrap_or(serde_json::Value::Null);
                let server_name = &self.meta.name;
                let server_version = self.meta.version.as_deref().unwrap_or("0.1.0");

                let mut result = serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": server_name,
                        "version": server_version
                    }
                });

                if let (Some(about), Some(obj)) = (
                    self.meta
                        .about
                        .as_deref()
                        .or(self.meta.long_about.as_deref()),
                    result.as_object_mut(),
                ) {
                    obj.insert(
                        "instructions".into(),
                        serde_json::Value::String(about.to_string()),
                    );
                }

                Some(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": result
                }))
            }
            "notifications/initialized" => {
                // MCP specification: notification sent after initialization, no response.
                None
            }
            "ping" => {
                let req_id = id.unwrap_or(serde_json::Value::Null);
                Some(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {}
                }))
            }
            "tools/list" => {
                let req_id = id.unwrap_or(serde_json::Value::Null);
                let tools = generate_tools_list(self.meta);
                Some(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "tools": tools
                    }
                }))
            }
            "tools/call" => {
                let req_id = id.unwrap_or(serde_json::Value::Null);
                let params = request.get("params");
                let tool_name = params
                    .and_then(|p| p.get("name"))
                    .and_then(serde_json::Value::as_str);

                let Some(name) = tool_name else {
                    return Some(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": "Missing 'name' in tools/call parameters"
                                }
                            ],
                            "isError": true
                        }
                    }));
                };

                let empty_args = serde_json::json!({});
                let arguments = params
                    .and_then(|p| p.get("arguments"))
                    .unwrap_or(&empty_args);

                match (self.runner)(name, arguments) {
                    Ok(output) => {
                        let text = if output.is_empty() {
                            "ok".to_string()
                        } else {
                            output
                        };
                        Some(serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "content": [
                                    {
                                        "type": "text",
                                        "text": text
                                    }
                                ],
                                "isError": false
                            }
                        }))
                    }
                    Err(failure) => Some(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": failure.message()
                                }
                            ],
                            "isError": true
                        }
                    })),
                }
            }
            unknown => id.map(|req_id| {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {unknown}")
                    }
                })
            }),
        }
    }

    /// Handles a single line of input (JSON-RPC request) and returns the JSON response string if any.
    #[must_use]
    pub fn handle_line(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            let err_resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "error": {
                    "code": -32700,
                    "message": "Parse error"
                }
            });
            return serde_json::to_string(&err_resp).ok();
        };

        let resp = self.handle_request(&value)?;
        serde_json::to_string(&resp).ok()
    }

    /// Runs the server loop over the given reader and writer until EOF.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if reading from `reader` or writing to `writer` fails.
    pub fn run<R: BufRead, W: Write>(&self, mut reader: R, mut writer: W) -> Result<(), Failure> {
        let mut line = String::new();
        while reader
            .read_line(&mut line)
            .map_err(|e| Failure::execution(e.to_string()))?
            > 0
        {
            if let Some(resp_line) = self.handle_line(&line) {
                writeln!(writer, "{resp_line}").map_err(|e| Failure::execution(e.to_string()))?;
                writer
                    .flush()
                    .map_err(|e| Failure::execution(e.to_string()))?;
            }
            line.clear();
        }
        Ok(())
    }
}

/// Generates the list of MCP tool definitions from [`CliMetadata`].
#[must_use]
pub fn generate_tools_list(meta: &CliMetadata) -> Vec<serde_json::Value> {
    let mut tools = Vec::new();
    collect_tools(meta, &[], &mut tools);
    tools
}

fn collect_tools(cmd: &CliMetadata, prefix: &[String], tools: &mut Vec<serde_json::Value>) {
    if !cmd.subcommands.is_empty() {
        for sub in &cmd.subcommands {
            if is_internal_command(&sub.name) {
                continue;
            }
            let mut next_prefix = prefix.to_vec();
            next_prefix.push(sub.name.clone());
            collect_tools(sub, &next_prefix, tools);
        }
    } else if !prefix.is_empty() {
        let tool_name = prefix.join("_");
        let desc = cmd
            .about
            .as_deref()
            .or(cmd.long_about.as_deref())
            .unwrap_or_default();
        let schema = command_arguments_to_input_schema(&cmd.arguments);

        tools.push(serde_json::json!({
            "name": tool_name,
            "description": desc,
            "inputSchema": schema,
        }));
    }
}

fn is_internal_command(name: &str) -> bool {
    matches!(name, "mcp" | "help" | "schema" | "write-skill")
}

fn command_arguments_to_input_schema(args: &[ArgumentMetadata]) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for arg in args {
        let mut prop = serde_json::Map::new();
        let val_type = arg.value_type.as_deref().unwrap_or(if arg.is_flag {
            "boolean"
        } else if arg.multiple {
            "array"
        } else {
            "string"
        });

        prop.insert(
            "type".into(),
            serde_json::Value::String(val_type.to_string()),
        );

        if let Some(help) = arg.help.as_deref().or(arg.long_help.as_deref()) {
            prop.insert(
                "description".into(),
                serde_json::Value::String(help.to_string()),
            );
        }

        if !arg.default_values.is_empty() {
            if arg.multiple {
                prop.insert("default".into(), serde_json::json!(arg.default_values));
            } else if let Some(first) = arg.default_values.first() {
                if arg.is_flag {
                    prop.insert("default".into(), serde_json::Value::Bool(first == "true"));
                } else if let Ok(num) = first.parse::<i64>() {
                    prop.insert("default".into(), serde_json::json!(num));
                } else if let Ok(float_num) = first.parse::<f64>() {
                    prop.insert("default".into(), serde_json::json!(float_num));
                } else {
                    prop.insert("default".into(), serde_json::Value::String(first.clone()));
                }
            }
        }

        if !arg.possible_values.is_empty() {
            prop.insert("enum".into(), serde_json::json!(arg.possible_values));
        }

        let key = arg.long.as_deref().unwrap_or(&arg.id);
        if arg.required {
            required.push(serde_json::Value::String(key.to_string()));
        }

        properties.insert(key.to_string(), serde_json::Value::Object(prop));
    }

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

/// Translates tool parameters into CLI argument tokens for execution.
///
/// # Errors
///
/// Returns a [`Failure`] if the tool command cannot be resolved in `meta`.
pub fn arguments_to_argv(
    cli_name: &str,
    tool_name: &str,
    arguments: &serde_json::Value,
    meta: &CliMetadata,
) -> Result<Vec<String>, Failure> {
    let (subcmd_names, target_meta) = find_subcommand_path(meta, tool_name)
        .ok_or_else(|| Failure::execution(format!("Tool '{tool_name}' not found")))?;

    let mut argv = Vec::new();
    argv.push(cli_name.to_string());
    argv.extend(subcmd_names);

    if arguments.is_null() {
        return Ok(argv);
    }

    if let Some(arr) = arguments.as_array() {
        for item in arr {
            argv.push(val_to_string(item));
        }
        return Ok(argv);
    }

    if let Some(obj) = arguments.as_object() {
        let mut options_args: Vec<String> = Vec::new();
        let mut positional_args: Vec<(usize, Vec<String>)> = Vec::new();

        for (key, val) in obj {
            if val.is_null() {
                continue;
            }

            let matched_arg = target_meta.arguments.iter().enumerate().find(|(_, a)| {
                a.id == *key
                    || a.long.as_deref() == Some(key.as_str())
                    || a.id.replace('_', "-") == key.replace('_', "-")
                    || a.long.as_deref().map(|l| l.replace('_', "-")) == Some(key.replace('_', "-"))
            });

            if let Some((idx, arg)) = matched_arg {
                if arg.positional {
                    if let Some(arr) = val.as_array() {
                        let strs: Vec<String> = arr.iter().map(val_to_string).collect();
                        positional_args.push((idx, strs));
                    } else {
                        positional_args.push((idx, vec![val_to_string(val)]));
                    }
                } else if arg.is_flag {
                    if val.as_bool() == Some(true) {
                        let flag = arg.long.as_deref().unwrap_or(&arg.id);
                        let flag_clean = flag.replace('_', "-");
                        options_args.push(format!("--{flag_clean}"));
                    }
                } else if arg.multiple {
                    let flag = arg.long.as_deref().unwrap_or(&arg.id);
                    let flag_clean = flag.replace('_', "-");
                    if let Some(arr) = val.as_array() {
                        for item in arr {
                            options_args.push(format!("--{flag_clean}"));
                            options_args.push(val_to_string(item));
                        }
                    } else {
                        options_args.push(format!("--{flag_clean}"));
                        options_args.push(val_to_string(val));
                    }
                } else {
                    let flag = arg.long.as_deref().unwrap_or(&arg.id);
                    let flag_clean = flag.replace('_', "-");
                    options_args.push(format!("--{flag_clean}"));
                    options_args.push(val_to_string(val));
                }
            } else {
                let flag_clean = key.trim_start_matches('-').replace('_', "-");
                if val.as_bool() == Some(true) {
                    options_args.push(format!("--{flag_clean}"));
                } else if val.as_bool() == Some(false) {
                    // Boolean false flag omitted.
                } else if let Some(arr) = val.as_array() {
                    for item in arr {
                        options_args.push(format!("--{flag_clean}"));
                        options_args.push(val_to_string(item));
                    }
                } else {
                    options_args.push(format!("--{flag_clean}"));
                    options_args.push(val_to_string(val));
                }
            }
        }

        argv.extend(options_args);
        positional_args.sort_by_key(|(idx, _)| *idx);
        for (_, strs) in positional_args {
            argv.extend(strs);
        }
    }

    Ok(argv)
}

fn find_subcommand_path<'m>(
    meta: &'m CliMetadata,
    tool_name: &str,
) -> Option<(Vec<String>, &'m CliMetadata)> {
    for sub in &meta.subcommands {
        if sub.name == tool_name || sub.name.replace('-', "_") == tool_name.replace('-', "_") {
            return Some((vec![sub.name.clone()], sub));
        }
    }

    let separators = ['_', ':', '-', ' '];
    for sep in separators {
        if tool_name.contains(sep) {
            let parts: Vec<&str> = tool_name.split(sep).collect();
            if let Some(res) = find_nested_path(meta, &parts) {
                return Some(res);
            }
        }
    }

    if meta.name == tool_name || meta.name.replace('-', "_") == tool_name.replace('-', "_") {
        return Some((Vec::new(), meta));
    }

    None
}

fn find_nested_path<'m>(
    curr: &'m CliMetadata,
    parts: &[&str],
) -> Option<(Vec<String>, &'m CliMetadata)> {
    if parts.is_empty() {
        return Some((Vec::new(), curr));
    }

    let head = parts[0];
    let tail = &parts[1..];

    for sub in &curr.subcommands {
        if sub.name == head || sub.name.replace('-', "_") == head.replace('-', "_") {
            if tail.is_empty() {
                return Some((vec![sub.name.clone()], sub));
            }
            if let Some((mut rest_names, leaf)) = find_nested_path(sub, tail) {
                rest_names.insert(0, sub.name.clone());
                return Some((rest_names, leaf));
            }
        }
    }
    None
}

fn val_to_string(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// Runs the MCP protocol over the provided reader and writer streams.
///
/// # Errors
///
/// Returns a [`Failure`] if reading or writing fails.
pub fn run_server_streams<R: BufRead, W: Write>(
    meta: &CliMetadata,
    runner: impl Fn(&str, &serde_json::Value) -> Result<String, Failure>,
    reader: R,
    writer: W,
) -> Result<String, Failure> {
    let server = McpServer::new(meta, runner);
    server.run(reader, writer)?;
    Ok(String::new())
}

/// Runs the MCP protocol over standard input and standard output streams.
///
/// # Errors
///
/// Returns a [`Failure`] if reading or writing fails.
pub fn run_stdio_server(
    meta: &CliMetadata,
    runner: impl Fn(&str, &serde_json::Value) -> Result<String, Failure>,
) -> Result<String, Failure> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run_server_streams(meta, runner, stdin.lock(), stdout.lock())
}
