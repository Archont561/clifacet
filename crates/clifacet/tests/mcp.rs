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
