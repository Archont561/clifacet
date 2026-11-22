#![cfg(feature = "mcp")]
#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use clifacet::mcp::{McpServer, arguments_to_argv, generate_tools_list, run_server_streams};
use clifacet::schema::CliMetadata;
use clifacet::{Failure, app, commands};
use std::io::Cursor;

#[app(
    name = "ops-tool",
    version = "1.2.0",
    about = "DevOps operational assistant CLI",
    commands
)]
struct OpsCli {
    #[clifacet(long)]
    pub env: Option<String>,
}

#[commands]
impl OpsCli {
    /// Ping service health check
    pub fn ping(&self) -> String {
        "pong".into()
    }

    /// Deploy container image to target cluster
    pub fn deploy(&self, image: String, #[clifacet(long)] replicas: Option<u32>) -> String {
        let r = replicas.unwrap_or(1);
        format!("deployed {image} with {r} replicas")
    }

    /// Echo message with uppercase option
    pub fn echo(&self, msg: String, #[clifacet(long)] uppercase: bool) -> String {
        if uppercase { msg.to_uppercase() } else { msg }
    }

    /// Failing command for error testing
    pub fn fail_op(&self) -> Result<String, Failure> {
        Err(Failure::execution("operation failed deliberately"))
    }
}

#[test]
fn test_mcp_initialize() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let server = McpServer::new(&meta, |_tool, _args| Ok("ok".into()));

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let resp = server
        .handle_request(&req)
        .expect("should produce response");
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);

    let result = &resp["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "ops-tool");
    assert_eq!(result["serverInfo"]["version"], "1.2.0");
    assert_eq!(result["instructions"], "DevOps operational assistant CLI");
    assert!(result["capabilities"]["tools"].is_object());
}

#[test]
fn test_mcp_notifications_initialized() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let server = McpServer::new(&meta, |_tool, _args| Ok("ok".into()));

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let resp = server.handle_request(&req);
    assert!(resp.is_none(), "notifications must not produce responses");
}

#[test]
fn test_mcp_ping() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let server = McpServer::new(&meta, |_tool, _args| Ok("ok".into()));

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "ping-req-42",
        "method": "ping"
    });

    let resp = server
        .handle_request(&req)
        .expect("should produce response");
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], "ping-req-42");
    assert_eq!(resp["result"], serde_json::json!({}));
}

#[test]
fn test_mcp_tools_list_reflects_controller_commands() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let tools = generate_tools_list(&meta);

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Controller commands must be present
    assert!(tool_names.contains(&"ping"));
    assert!(tool_names.contains(&"deploy"));
    assert!(tool_names.contains(&"echo"));
    assert!(tool_names.contains(&"fail-op") || tool_names.contains(&"fail_op"));

    // Internal commands must not be exposed as tools
    assert!(!tool_names.contains(&"mcp"));
    assert!(!tool_names.contains(&"help"));
    assert!(!tool_names.contains(&"schema"));
    assert!(!tool_names.contains(&"write-skill"));

    // Verify deploy schema
    let deploy_tool = tools.iter().find(|t| t["name"] == "deploy").unwrap();
    assert_eq!(
        deploy_tool["description"],
        "Deploy container image to target cluster"
    );
    let schema = &deploy_tool["inputSchema"];
    assert_eq!(schema["type"], "object");

    let props = &schema["properties"];
    assert!(props.get("image").is_some());
    assert_eq!(props["image"]["type"], "string");

    let required = schema["required"].as_array().unwrap();
    assert!(required.iter().any(|r| r == "image"));
}

#[test]
fn test_mcp_arguments_to_argv_conversion() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    // Positional + optional named argument
    let args = serde_json::json!({
        "image": "redis:7.0",
        "replicas": 4
    });
    let argv = arguments_to_argv("ops-tool", "deploy", &args, &meta).unwrap();
    assert_eq!(argv[0], "ops-tool");
    assert_eq!(argv[1], "deploy");
    assert!(argv.contains(&"--replicas".to_string()));
    assert!(argv.contains(&"4".to_string()));
    assert!(argv.contains(&"redis:7.0".to_string()));

    // Boolean flag
    let echo_args = serde_json::json!({
        "msg": "hello mcp",
        "uppercase": true
    });
    let echo_argv = arguments_to_argv("ops-tool", "echo", &echo_args, &meta).unwrap();
    assert!(echo_argv.contains(&"--uppercase".to_string()));
    assert!(echo_argv.contains(&"hello mcp".to_string()));

    // Boolean flag false (should not pass flag)
    let echo_false = serde_json::json!({
        "msg": "hello mcp",
        "uppercase": false
    });
    let echo_false_argv = arguments_to_argv("ops-tool", "echo", &echo_false, &meta).unwrap();
    assert!(!echo_false_argv.contains(&"--uppercase".to_string()));
}

#[test]
fn test_mcp_tools_call_success_and_failure() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    let runner = move |tool_name: &str, args: &serde_json::Value| -> Result<String, Failure> {
        let argv = arguments_to_argv(&meta.name, tool_name, args, &meta)?;
        let app = clifacet::App::<OpsCli>::for_cli();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let exit_code = app.run_with(
            argv,
            clifacet::policy::Capabilities::headless(),
            || Box::new(clifacet::prompt::Scripted::default()),
            &mut out,
            &mut err,
        );
        if exit_code == 0 {
            let s = String::from_utf8(out).map_err(|e| Failure::execution(e.to_string()))?;
            Ok(s.trim().to_string())
        } else {
            let s = String::from_utf8(err).map_err(|e| Failure::execution(e.to_string()))?;
            let msg = if s.trim().is_empty() {
                let out_s = String::from_utf8(out).unwrap_or_default();
                if out_s.trim().is_empty() {
                    format!("exit code {exit_code}")
                } else {
                    out_s.trim().to_string()
                }
            } else {
                s.trim().to_string()
            };
            Err(Failure::execution(msg))
        }
    };

    let server_meta = CliMetadata::from_command(&cmd);
    let server = McpServer::new(&server_meta, runner);

    // 1. Success tool call: ping
    let ping_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "ping",
            "arguments": {}
        }
    });
    let ping_resp = server.handle_request(&ping_req).unwrap();
    assert_eq!(ping_resp["id"], 10);
    assert_eq!(ping_resp["result"]["isError"], false);
    assert_eq!(ping_resp["result"]["content"][0]["text"], "pong");

    // 2. Success tool call: deploy with parameters
    let deploy_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "deploy",
            "arguments": {
                "image": "nginx:alpine",
                "replicas": 3
            }
        }
    });
    let deploy_resp = server.handle_request(&deploy_req).unwrap();
    assert_eq!(deploy_resp["id"], 11);
    assert_eq!(deploy_resp["result"]["isError"], false);
    assert_eq!(
        deploy_resp["result"]["content"][0]["text"],
        "deployed nginx:alpine with 3 replicas"
    );

    // 3. Tool call with validation error: missing required argument
    let missing_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "tools/call",
        "params": {
            "name": "deploy",
            "arguments": {
                "replicas": 3
            }
        }
    });
    let missing_resp = server.handle_request(&missing_req).unwrap();
    assert_eq!(missing_resp["id"], 12);
    assert_eq!(missing_resp["result"]["isError"], true);
    let err_msg = missing_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(err_msg.contains("required") || err_msg.contains("IMAGE"));

    // 4. Command execution failure
    let fail_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "tools/call",
        "params": {
            "name": "fail-op",
            "arguments": {}
        }
    });
    let fail_resp = server.handle_request(&fail_req).unwrap();
    assert_eq!(fail_resp["id"], 13);
    assert_eq!(fail_resp["result"]["isError"], true);
    assert!(
        fail_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("operation failed deliberately")
    );

    // 5. Unknown tool call
    let unknown_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "tools/call",
        "params": {
            "name": "non_existent_tool",
            "arguments": {}
        }
    });
    let unknown_resp = server.handle_request(&unknown_req).unwrap();
    assert_eq!(unknown_resp["id"], 14);
    assert_eq!(unknown_resp["result"]["isError"], true);
    assert!(
        unknown_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

#[test]
fn test_mcp_jsonrpc_protocol_errors() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let server = McpServer::new(&meta, |_t, _a| Ok("ok".into()));

    // Invalid JSON
    let invalid_json = server.handle_line("not-valid-json\n");
    let resp: serde_json::Value =
        serde_json::from_str(&invalid_json.expect("should return error")).unwrap();
    assert_eq!(resp["error"]["code"], -32700);

    // Unknown method
    let unknown_method = server
        .handle_line(r#"{"jsonrpc": "2.0", "id": 99, "method": "prompts/list"}"#)
        .expect("should return error");
    let resp: serde_json::Value = serde_json::from_str(&unknown_method).unwrap();
    assert_eq!(resp["id"], 99);
    assert_eq!(resp["error"]["code"], -32601);

    // Missing JSON-RPC version
    let bad_version = server
        .handle_line(r#"{"id": 100, "method": "ping"}"#)
        .expect("should return error");
    let resp: serde_json::Value = serde_json::from_str(&bad_version).unwrap();
    assert_eq!(resp["id"], 100);
    assert_eq!(resp["error"]["code"], -32600);
}

#[test]
fn test_mcp_stdio_streams_conversation() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    let requests = [
        r#"{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}}"#,
        r#"{"jsonrpc": "2.0", "method": "notifications/initialized"}"#,
        r#"{"jsonrpc": "2.0", "id": 2, "method": "ping"}"#,
        r#"{"jsonrpc": "2.0", "id": 3, "method": "tools/list"}"#,
        r#"{"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "deploy", "arguments": {"image": "caddy:2", "replicas": 2}}}"#,
    ];

    let input_bytes = requests.join("\n") + "\n";
    let reader = Cursor::new(input_bytes.into_bytes());
    let mut writer = Vec::new();

    let runner_meta = meta.clone();
    let runner = move |tool_name: &str, args: &serde_json::Value| -> Result<String, Failure> {
        let argv = arguments_to_argv(&runner_meta.name, tool_name, args, &runner_meta)?;
        let app = clifacet::App::<OpsCli>::for_cli();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let exit_code = app.run_with(
            argv,
            clifacet::policy::Capabilities::headless(),
            || Box::new(clifacet::prompt::Scripted::default()),
            &mut out,
            &mut err,
        );
        if exit_code == 0 {
            Ok(String::from_utf8(out).unwrap().trim().to_string())
        } else {
            Err(Failure::execution("error"))
        }
    };

    run_server_streams(&meta, runner, reader, &mut writer).expect("streams run must succeed");

    let output_str = String::from_utf8(writer).expect("valid utf-8 output");
    let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();

    // 4 responses expected (notification produced 0)
    assert_eq!(lines.len(), 4);

    let resp1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(resp1["id"], 1);
    assert_eq!(resp1["result"]["protocolVersion"], "2024-11-05");

    let resp2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(resp2["id"], 2);

    let resp3: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(resp3["id"], 3);
    assert!(resp3["result"]["tools"].as_array().unwrap().len() >= 3);

    let resp4: serde_json::Value = serde_json::from_str(lines[3]).unwrap();
    assert_eq!(resp4["id"], 4);
    assert_eq!(resp4["result"]["isError"], false);
    assert_eq!(
        resp4["result"]["content"][0]["text"],
        "deployed caddy:2 with 2 replicas"
    );
}

#[test]
fn test_mcp_cli_command_invocation() {
    let app = clifacet::App::<OpsCli>::for_cli();

    // Verify `ops-tool --help` displays `mcp` subcommand
    let mut out = Vec::new();
    let mut err = Vec::new();
    let exit_code = app.run_with(
        ["ops-tool", "--help"],
        clifacet::policy::Capabilities::headless(),
        || Box::new(clifacet::prompt::Scripted::default()),
        &mut out,
        &mut err,
    );
    assert_eq!(exit_code, 0);
    let help_str = String::from_utf8(out).unwrap();
    assert!(help_str.contains("mcp"));
    assert!(help_str.contains("Run Model Context Protocol (MCP) stdio tool server"));
}

#[test]
fn test_mcp_inherent_run_mcp_server_streams() {
    let requests = [
        r#"{"jsonrpc": "2.0", "id": 1, "method": "initialize"}"#,
        r#"{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "ping", "arguments": {}}}"#,
    ];
    let input = requests.join("\n") + "\n";
    let reader = Cursor::new(input.into_bytes());
    let mut writer = Vec::new();

    OpsCli::run_mcp_server_streams(reader, &mut writer).expect("inherent streams run must succeed");

    let out = String::from_utf8(writer).unwrap();
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);

    let resp1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(resp1["result"]["serverInfo"]["name"], "ops-tool");

    let resp2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(resp2["result"]["content"][0]["text"], "pong");
}

#[clifacet::group(commands)]
pub struct AdminGroup {}

#[commands(group)]
impl AdminGroup {
    /// Flush system cache
    pub fn flush(&self) -> Result<String, Failure> {
        Ok("cache flushed successfully".into())
    }
}

#[app(name = "nested-cli", about = "CLI with nested commands", commands)]
pub struct NestedCli {}

#[commands]
impl NestedCli {
    /// Root level status
    pub fn status(&self) -> Result<String, Failure> {
        Ok("all systems operational".into())
    }

    /// Mount admin command group
    pub fn admin(&self, #[clifacet(subcommand)] admin: AdminGroup) -> Result<String, Failure> {
        admin.execute()
    }
}

#[test]
fn test_mcp_nested_subcommand_tools() {
    let cmd = <NestedCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let tools = generate_tools_list(&meta);

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(tool_names.contains(&"status"));
    assert!(tool_names.contains(&"admin_flush"));

    let requests = [
        r#"{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "admin_flush", "arguments": {}}}"#,
        r#"{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "status", "arguments": {}}}"#,
    ];
    let input = requests.join("\n") + "\n";
    let reader = Cursor::new(input.into_bytes());
    let mut writer = Vec::new();

    NestedCli::run_mcp_server_streams(reader, &mut writer).expect("streams run succeeded");

    let out = String::from_utf8(writer).unwrap();
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);

    let resp1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(resp1["id"], 1);
    assert_eq!(resp1["result"]["isError"], false);
    assert_eq!(
        resp1["result"]["content"][0]["text"],
        "cache flushed successfully"
    );

    let resp2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(resp2["id"], 2);
    assert_eq!(resp2["result"]["isError"], false);
    assert_eq!(
        resp2["result"]["content"][0]["text"],
        "all systems operational"
    );
}

#[app(
    name = "types-cli",
    about = "CLI with rich types and defaults",
    commands
)]
pub struct TypesCli {}

#[commands]
impl TypesCli {
    /// Command with diverse default values
    pub fn config(
        &self,
        #[clifacet(long, default_value = "main")] cluster: String,
        #[clifacet(long, default_value = "true")] enabled: bool,
        #[clifacet(long, default_value = "42")] retries: i64,
        #[clifacet(long, default_value = "2.5")] rate: f64,
        #[clifacet(long)] tags: Vec<String>,
    ) -> String {
        format!("{cluster}-{enabled}-{retries}-{rate}-{tags:?}")
    }
}

#[test]
fn test_mcp_schema_defaults_and_array_args() {
    let cmd = <TypesCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let tools = generate_tools_list(&meta);

    let config_tool = tools.iter().find(|t| t["name"] == "config").unwrap();
    let props = &config_tool["inputSchema"]["properties"];

    assert_eq!(props["cluster"]["default"], "main");
    assert_eq!(props["enabled"]["default"], true);
    assert_eq!(props["retries"]["default"], 42);
    assert_eq!(props["rate"]["default"], 2.5);

    // Test arguments_to_argv with multiple option as array and single item
    let arr_args = serde_json::json!({
        "cluster": "prod",
        "tags": ["env:prod", "tier:frontend"]
    });
    let argv = arguments_to_argv("types-cli", "config", &arr_args, &meta).unwrap();
    assert!(argv.contains(&"--cluster".to_string()));
    assert!(argv.contains(&"prod".to_string()));
    assert!(argv.contains(&"--tags".to_string()));
    assert!(argv.contains(&"env:prod".to_string()));
    assert!(argv.contains(&"tier:frontend".to_string()));

    let single_tag_args = serde_json::json!({
        "tags": "env:staging"
    });
    let argv2 = arguments_to_argv("types-cli", "config", &single_tag_args, &meta).unwrap();
    assert!(argv2.contains(&"env:staging".to_string()));

    // Test array arguments (positional list)
    let raw_arr = serde_json::json!(["pos1", "pos2"]);
    let argv3 = arguments_to_argv("types-cli", "config", &raw_arr, &meta).unwrap();
    assert_eq!(argv3, vec!["types-cli", "config", "pos1", "pos2"]);

    // Test unknown options with various types
    let unknown_args = serde_json::json!({
        "extra_flag": true,
        "ignored_flag": false,
        "extra_arr": ["x1", "x2"],
        "extra_val": "hello"
    });
    let argv4 = arguments_to_argv("types-cli", "config", &unknown_args, &meta).unwrap();
    assert!(argv4.contains(&"--extra-flag".to_string()));
    assert!(!argv4.contains(&"--ignored-flag".to_string()));
    assert!(argv4.contains(&"--extra-arr".to_string()));
    assert!(argv4.contains(&"x1".to_string()));
    assert!(argv4.contains(&"x2".to_string()));
    assert!(argv4.contains(&"--extra-val".to_string()));
    assert!(argv4.contains(&"hello".to_string()));
}

#[test]
fn test_mcp_missing_method_and_missing_tool_name() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let server = McpServer::new(&meta, |_t, _a| Ok("ok".into()));

    // Request missing "method"
    let no_method_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 88
    });
    let resp = server.handle_request(&no_method_req).unwrap();
    assert_eq!(resp["id"], 88);
    assert_eq!(resp["error"]["code"], -32600);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("missing method")
    );

    // tools/call missing "name" parameter
    let no_name_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 89,
        "method": "tools/call",
        "params": {
            "arguments": {}
        }
    });
    let resp2 = server.handle_request(&no_name_req).unwrap();
    assert_eq!(resp2["id"], 89);
    assert_eq!(resp2["result"]["isError"], true);
    assert!(
        resp2["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing 'name'")
    );
}

// ---------------------------------------------------------------------------
// CLI-86: MCP Async Dispatch, Cancellation, Progress, and Concurrent Calls
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct AsyncDeployCmd;

impl clifacet::r#async::AsyncHandler<()> for AsyncDeployCmd {
    type Input = (String, u32);
    type Output = String;

    async fn handle(
        &self,
        (image, replicas): Self::Input,
        _cx: &(),
        run: &clifacet::r#async::RunCx,
    ) -> Result<Self::Output, Failure> {
        run.progress().total(Some(100));
        run.progress().advance(50);
        run.progress().message("deploying pods");
        run.progress().advance(50);
        Ok(format!(
            "deployed {image} with {replicas} replicas asynchronously"
        ))
    }
}

#[tokio::test]
async fn test_mcp_async_handler_dispatch_and_progress_streaming() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    let deploy_handler = AsyncDeployCmd;
    let runner =
        move |tool_name: &str, args: &serde_json::Value, run_cx: &clifacet::r#async::RunCx| {
            let tool_name = tool_name.to_string();
            let args = args.clone();
            let run_cx = run_cx.clone();
            async move {
                if tool_name == "deploy" {
                    let image = args["image"].as_str().unwrap_or("app:1").to_string();
                    let replicas = args["replicas"].as_u64().unwrap_or(1) as u32;
                    clifacet::mcp::dispatch_async_handler(
                        &deploy_handler,
                        (image, replicas),
                        &(),
                        &run_cx,
                    )
                    .await
                } else {
                    Err(Failure::execution("unknown tool"))
                }
            }
        };

    let requests = [
        r#"{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}}"#,
        r#"{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "deploy", "arguments": {"image": "nginx:alpine", "replicas": 3}, "_meta": {"progressToken": "token-123"}}}"#,
    ];
    let input_bytes = requests.join("\n") + "\n";
    let reader = Cursor::new(input_bytes.into_bytes());
    let mut writer = Vec::new();

    clifacet::mcp::run_server_streams_async(&meta, runner, reader, &mut writer)
        .await
        .expect("async streams run should succeed");

    let output_str = String::from_utf8(writer).expect("valid utf-8 output");
    let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();

    // Line 0: initialize response
    let init_resp: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(init_resp["id"], 1);

    // Should include progress notifications (AC #4)
    let progress_lines: Vec<serde_json::Value> = lines
        .iter()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v.get("method").and_then(|m| m.as_str()) == Some("notifications/progress"))
        .collect();

    assert!(
        !progress_lines.is_empty(),
        "must have emitted progress notifications"
    );
    let p_last = &progress_lines[progress_lines.len() - 1];
    assert_eq!(p_last["params"]["progressToken"], "token-123");
    assert_eq!(p_last["params"]["progress"], 100);
    assert_eq!(p_last["params"]["total"], 100);

    // Final line should be tool call response (AC #1)
    let call_resp: serde_json::Value = serde_json::from_str(lines.last().unwrap()).unwrap();
    assert_eq!(call_resp["id"], 2);
    assert_eq!(call_resp["result"]["isError"], false);
    assert_eq!(
        call_resp["result"]["content"][0]["text"],
        "deployed nginx:alpine with 3 replicas asynchronously"
    );
}

#[tokio::test]
async fn test_mcp_async_cancellation_notification() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    let runner =
        move |_tool_name: &str, _args: &serde_json::Value, run_cx: &clifacet::r#async::RunCx| {
            let run_cx = run_cx.clone();
            async move {
                // Wait up to 500ms or until cancelled
                tokio::select! {
                    () = run_cx.cancel().cancelled() => {
                        Err(Failure::cancelled("tool call was cancelled by client"))
                    }
                    () = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                        Ok("completed without cancel".to_string())
                    }
                }
            }
        };

    let requests = [
        r#"{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}}"#,
        r#"{"jsonrpc": "2.0", "id": 42, "method": "tools/call", "params": {"name": "deploy", "arguments": {}}}"#,
        r#"{"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 42}}"#,
    ];
    let input_bytes = requests.join("\n") + "\n";
    let reader = Cursor::new(input_bytes.into_bytes());
    let mut writer = Vec::new();

    clifacet::mcp::run_server_streams_async(&meta, runner, reader, &mut writer)
        .await
        .expect("async streams run should succeed");

    let output_str = String::from_utf8(writer).expect("valid utf-8");
    let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();

    let call_resp: serde_json::Value = serde_json::from_str(lines.last().unwrap()).unwrap();
    assert_eq!(call_resp["id"], 42);
    assert_eq!(call_resp["result"]["isError"], true);
    assert!(
        call_resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("cancelled by client")
    );
}

struct SyncComputeCmd;

impl clifacet::Handler<u32> for SyncComputeCmd {
    type Input = u32;
    type Output = String;

    fn handle(&self, input: Self::Input, cx: &u32) -> Result<Self::Output, Failure> {
        Ok(format!("computed: {}", input * cx))
    }
}

#[tokio::test]
async fn test_mcp_sync_handler_spawn_blocking_dispatch() {
    let cx = std::sync::Arc::new(10u32);
    let handler = SyncComputeCmd;

    // AC #2: Synchronous handler executed safely via tokio::task::spawn_blocking
    let res = clifacet::mcp::dispatch_sync_handler(handler, 5, cx).await;
    assert_eq!(res.unwrap(), "computed: 50");
}

#[tokio::test]
async fn test_mcp_concurrent_tool_calls_sharing_context() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let runner_counter = counter.clone();

    let runner =
        move |_tool_name: &str, _args: &serde_json::Value, _run_cx: &clifacet::r#async::RunCx| {
            let counter = runner_counter.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                let val = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(format!("call-{val}"))
            }
        };

    let requests = [
        r#"{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}}"#,
        r#"{"jsonrpc": "2.0", "id": 10, "method": "tools/call", "params": {"name": "ping", "arguments": {}}}"#,
        r#"{"jsonrpc": "2.0", "id": 11, "method": "tools/call", "params": {"name": "ping", "arguments": {}}}"#,
        r#"{"jsonrpc": "2.0", "id": 12, "method": "tools/call", "params": {"name": "ping", "arguments": {}}}"#,
    ];
    let input_bytes = requests.join("\n") + "\n";
    let reader = Cursor::new(input_bytes.into_bytes());
    let mut writer = Vec::new();

    clifacet::mcp::run_server_streams_async(&meta, runner, reader, &mut writer)
        .await
        .expect("async streams run should succeed");

    let output_str = String::from_utf8(writer).expect("valid utf-8");
    let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();

    // 1 init response + 3 tool call responses = 4 lines
    assert_eq!(lines.len(), 4);
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_mcp_progress_sink_unit_methods() {
    use clifacet::r#async::ProgressSink;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let sink = clifacet::mcp::McpProgressSink::new(serde_json::json!("tok-1"), tx);

    sink.total(None);
    sink.total(Some(100));
    sink.advance(25);
    sink.message("step 1");

    let mut messages = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        messages.push(msg);
    }
    assert_eq!(messages.len(), 4);
    assert!(messages[0].contains("tok-1"));
    assert!(messages[3].contains("step 1"));
}

#[tokio::test]
async fn test_mcp_async_protocol_edge_cases() {
    let cmd = <OpsCli as clifacet::Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    let runner =
        |_tool_name: &str, _args: &serde_json::Value, _run_cx: &clifacet::r#async::RunCx| async {
            Ok("done".to_string())
        };

    let requests = [
        "not json at all\n",
        "\n",
        r#"{"jsonrpc": "2.0", "id": 5, "method": "unsupported/method", "params": {}}"#,
        r#"{"jsonrpc": "2.0", "id": "custom-id", "method": "tools/call", "params": {}}"#,
        r#"{"jsonrpc": "2.0", "id": true, "method": "tools/call", "params": {"name": "deploy", "arguments": {}}}"#,
    ];
    let input_bytes = requests.join("\n") + "\n";
    let reader = Cursor::new(input_bytes.into_bytes());
    let mut writer = Vec::new();

    clifacet::mcp::run_server_streams_async(&meta, runner, reader, &mut writer)
        .await
        .expect("should succeed");

    let output_str = String::from_utf8(writer).expect("valid utf-8");
    assert!(output_str.contains("-32700")); // Parse error
    assert!(output_str.contains("-32601")); // Method not found
    assert!(output_str.contains("Missing 'name'"));
    assert!(output_str.contains("done"));
}

#[tokio::test]
async fn test_mcp_sync_handler_failure() {
    struct FailCmd;

    impl clifacet::Handler<()> for FailCmd {
        type Input = ();
        type Output = ();

        fn handle(&self, _: Self::Input, _: &()) -> Result<Self::Output, Failure> {
            Err(Failure::execution("deliberate failure"))
        }
    }

    let cx = std::sync::Arc::new(());
    let res = clifacet::mcp::dispatch_sync_handler(FailCmd, (), cx).await;
    assert!(res.is_err());
    assert!(res.unwrap_err().message().contains("deliberate failure"));
}
