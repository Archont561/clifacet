//! Integration tests for machine-readable CLI schema introspection (CLI-73, decision-25).

#![cfg(feature = "schema")]
#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use clap::{Parser, Subcommand};
use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, CliMetadata, Facet, Handler, app, commands};
use serde_json::Value;

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(clap::ValueEnum, Debug, Clone, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Yaml,
    Text,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "testcli",
    about = "A test CLI application",
    version = "1.2.3",
    author = "Test Author"
)]
struct TestCli {
    /// Verbose output flag
    #[arg(short, long)]
    verbose: bool,

    /// Target environment
    #[arg(long, default_value = "production", env = "TESTCLI_ENV")]
    env: String,

    /// Port to listen on
    #[arg(short, long, value_name = "PORT")]
    port: Option<u16>,

    /// Positional input file
    #[arg(required = true)]
    input_file: String,

    /// Output format choice
    #[arg(long, value_enum, default_value = "json")]
    format: OutputFormat,

    /// Tags applied to output
    #[arg(long = "tag", action = clap::ArgAction::Append)]
    tags: Vec<String>,

    #[command(subcommand)]
    command: Option<TestCommands>,
}

#[derive(Subcommand, Debug, Clone)]
enum TestCommands {
    /// Start the service
    Start {
        /// Number of workers
        #[arg(long, default_value = "4")]
        workers: u32,

        #[command(subcommand)]
        mode: Option<StartMode>,
    },
    /// Stop the service
    Stop {
        /// Force stop immediately
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum StartMode {
    /// Run as a daemon in the background
    Daemon {
        /// PID file path
        #[arg(long, default_value = "/var/run/test.pid")]
        pid_file: String,
    },
    /// Run in foreground
    Foreground,
}

impl clifacet::prompt::Collect for TestCli {
    fn collect(self, _session: &mut clifacet::prompt::Session) -> Result<Self, clifacet::Failure> {
        Ok(self)
    }
}

impl Facet for TestCli {
    type Input = Self;

    fn command() -> clap::Command {
        <Self as clap::CommandFactory>::command()
    }

    fn validate(self) -> Result<Self::Input, clifacet::Failure> {
        Ok(self)
    }
}

impl clifacet::Cli for TestCli {
    type Context = ();
    fn init_context(&self) -> Result<Self::Context, clifacet::Failure> {
        Ok(())
    }
}

impl Handler<()> for TestCli {
    type Input = Self;
    type Output = String;

    fn handle(&self, _input: Self::Input, _cx: &()) -> Result<Self::Output, clifacet::Failure> {
        Ok("executed".to_string())
    }
}

#[test]
fn test_cli_metadata_reflection() {
    let cmd = <TestCli as Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    assert_eq!(meta.name, "testcli");
    assert_eq!(meta.about.as_deref(), Some("A test CLI application"));
    assert_eq!(meta.version.as_deref(), Some("1.2.3"));
    assert_eq!(meta.author.as_deref(), Some("Test Author"));

    // Check flag argument
    let verbose_arg = meta
        .arguments
        .iter()
        .find(|a| a.id == "verbose")
        .expect("verbose arg");
    assert!(verbose_arg.is_flag);
    assert_eq!(verbose_arg.short, Some('v'));
    assert_eq!(verbose_arg.long.as_deref(), Some("verbose"));
    assert_eq!(verbose_arg.help.as_deref(), Some("Verbose output flag"));
    assert_eq!(verbose_arg.value_type.as_deref(), Some("boolean"));
    assert!(!verbose_arg.required);
    assert!(!verbose_arg.positional);

    // Check option with default and env
    let env_arg = meta
        .arguments
        .iter()
        .find(|a| a.id == "env")
        .expect("env arg");
    assert!(!env_arg.is_flag);
    assert_eq!(env_arg.long.as_deref(), Some("env"));
    assert_eq!(env_arg.env.as_deref(), Some("TESTCLI_ENV"));
    assert_eq!(env_arg.default_values, vec!["production"]);

    // Check required positional argument
    let file_arg = meta
        .arguments
        .iter()
        .find(|a| a.id == "input_file")
        .expect("input_file arg");
    assert!(file_arg.positional);
    assert!(file_arg.required);
    assert_eq!(file_arg.help.as_deref(), Some("Positional input file"));

    // Check enum/possible values argument
    let format_arg = meta
        .arguments
        .iter()
        .find(|a| a.id == "format")
        .expect("format arg");
    assert_eq!(format_arg.possible_values, vec!["json", "yaml", "text"]);
    assert_eq!(format_arg.default_values, vec!["json"]);

    // Check multiple/array argument
    let tags_arg = meta
        .arguments
        .iter()
        .find(|a| a.id == "tags")
        .expect("tags arg");
    assert!(tags_arg.multiple);
    assert_eq!(tags_arg.value_type.as_deref(), Some("array"));

    // Check nested subcommands
    assert_eq!(meta.subcommands.len(), 2);
    let start_cmd = meta
        .subcommands
        .iter()
        .find(|s| s.name == "start")
        .expect("start cmd");
    assert_eq!(start_cmd.about.as_deref(), Some("Start the service"));

    let workers_arg = start_cmd
        .arguments
        .iter()
        .find(|a| a.id == "workers")
        .expect("workers arg");
    assert_eq!(workers_arg.default_values, vec!["4"]);

    // Check 2nd level subcommands
    assert_eq!(start_cmd.subcommands.len(), 2);
    let daemon_sub = start_cmd
        .subcommands
        .iter()
        .find(|s| s.name == "daemon")
        .expect("daemon subcommand");
    assert_eq!(
        daemon_sub.about.as_deref(),
        Some("Run as a daemon in the background")
    );
    let pid_arg = daemon_sub
        .arguments
        .iter()
        .find(|a| a.id == "pid_file")
        .expect("pid_file arg");
    assert_eq!(pid_arg.default_values, vec!["/var/run/test.pid"]);
}

#[test]
fn test_cli_metadata_serde_roundtrip() {
    let cmd = <TestCli as Facet>::command();
    let meta = CliMetadata::from_command(&cmd);

    let serialized = serde_json::to_string_pretty(&meta).expect("serialized");
    let deserialized: CliMetadata = serde_json::from_str(&serialized).expect("deserialized");

    assert_eq!(meta, deserialized);
}

#[test]
fn test_cli_to_json_schema() {
    let cmd = <TestCli as Facet>::command();
    let meta = CliMetadata::from_command(&cmd);
    let schema = meta.to_json_schema();

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["title"], "testcli");
    assert_eq!(schema["description"], "A test CLI application");
    assert_eq!(schema["type"], "object");

    let props = &schema["properties"];
    assert!(props["verbose"].is_object());
    assert_eq!(props["verbose"]["type"], "boolean");
    assert_eq!(props["verbose"]["description"], "Verbose output flag");
    assert_eq!(props["verbose"]["flag"], "--verbose");
    assert_eq!(props["verbose"]["short_flag"], "-v");

    assert!(props["env"].is_object());
    assert_eq!(props["env"]["type"], "string");
    assert_eq!(props["env"]["environment_variable"], "TESTCLI_ENV");
    assert_eq!(props["env"]["default"], "production");

    assert_eq!(
        props["format"]["enum"],
        serde_json::json!(["json", "yaml", "text"])
    );
    assert_eq!(props["format"]["default"], "json");

    assert_eq!(props["tags"]["type"], "array");

    // Required arguments check
    let required = schema["required"].as_array().expect("required array");
    assert!(required.contains(&serde_json::Value::String("input_file".to_string())));

    let subcmds = schema["subcommands"].as_array().expect("subcommands array");
    assert_eq!(subcmds.len(), 2);
    let start_sub = subcmds
        .iter()
        .find(|s| s["title"] == "start")
        .expect("start subcommand");
    assert_eq!(start_sub["description"], "Start the service");
    assert_eq!(start_sub["properties"]["workers"]["default"], "4");

    let start_sub_subcmds = start_sub["subcommands"]
        .as_array()
        .expect("start subcommands");
    assert_eq!(start_sub_subcmds.len(), 2);
    let daemon_sub = start_sub_subcmds
        .iter()
        .find(|s| s["title"] == "daemon")
        .expect("daemon");
    assert_eq!(
        daemon_sub["description"],
        "Run as a daemon in the background"
    );
}

#[test]
fn test_app_run_with_schema_flag() {
    let app = App::<TestCli, TestCli, clifacet::Plain, ()>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit = app.run_with(
        ["testcli", "--schema"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(exit, 0);
    assert!(err.is_empty());

    let output_str = String::from_utf8(out).expect("valid utf8");
    let schema_json: Value = serde_json::from_str(&output_str).expect("valid json schema");
    assert_eq!(schema_json["title"], "testcli");
    assert_eq!(schema_json["properties"]["verbose"]["type"], "boolean");
}

#[test]
fn test_app_run_with_schema_subcommand() {
    let app = App::<TestCli, TestCli, clifacet::Plain, ()>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit = app.run_with(
        ["testcli", "schema"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(exit, 0);
    assert!(err.is_empty());

    let output_str = String::from_utf8(out).expect("valid utf8");
    let schema_json: Value = serde_json::from_str(&output_str).expect("valid json schema");
    assert_eq!(schema_json["title"], "testcli");
    assert_eq!(
        schema_json["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
}

#[test]
fn test_app_run_with_schema_help() {
    let app = App::<ControllerApp, ControllerApp, clifacet::Plain, ()>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    // Invoking `controller-app --help` should display the generated schema subcommand
    let exit = app.run_with(
        ["controller-app", "--help"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(exit, 0);
    let output_str = String::from_utf8(out).expect("valid utf8");
    assert!(
        output_str.contains("schema"),
        "help text should mention schema subcommand: {output_str}"
    );

    // Invoking `controller-app schema --help` should display help for the schema command
    let mut out_sub = Vec::new();
    let mut err_sub = Vec::new();
    let exit_sub = app.run_with(
        ["controller-app", "schema", "--help"],
        Capabilities::headless(),
        prompter,
        &mut out_sub,
        &mut err_sub,
    );

    assert_eq!(exit_sub, 0);
    let sub_help = String::from_utf8(out_sub).expect("valid utf8");
    assert!(
        sub_help.contains("Print the machine-readable CLI JSON schema"),
        "subcommand help text: {sub_help}"
    );
}

#[app(name = "globals-app", about = "App with globals")]
struct GlobalsApp {
    #[command(flatten)]
    pub globals: clifacet::Globals,
}

impl Handler<()> for GlobalsApp {
    type Input = Self;
    type Output = String;
    fn handle(&self, _input: Self::Input, _cx: &()) -> Result<Self::Output, clifacet::Failure> {
        Ok("ok".to_string())
    }
}

#[test]
fn test_app_with_globals_shows_schema_flag_in_help() {
    let app = App::<GlobalsApp, GlobalsApp, clifacet::Plain, ()>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit = app.run_with(
        ["globals-app", "--help"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(exit, 0);
    let output_str = String::from_utf8(out).expect("valid utf8");
    assert!(
        output_str.contains("--schema"),
        "help text should contain --schema flag: {output_str}"
    );
}

#[app(
    name = "controller-app",
    about = "Controller-based application",
    commands
)]
struct ControllerApp {
    /// An app flag
    #[clifacet(long)]
    pub debug: bool,
}

#[commands]
impl ControllerApp {
    /// Ping the service
    pub fn ping(&self) -> &'static str {
        "pong"
    }

    /// Status check with message
    pub fn status(&self, message: String) -> Result<String, clifacet::Failure> {
        Ok(message)
    }
}

#[test]
fn test_app_macro_generates_schema_methods_and_subcommand() {
    // Check inherent methods generated by #[clifacet::app]
    let meta = ControllerApp::cli_metadata();
    assert_eq!(meta.name, "controller-app");
    assert_eq!(meta.about.as_deref(), Some("Controller-based application"));

    let schema_json_str = ControllerApp::cli_schema_json().expect("schema json");
    let parsed: Value = serde_json::from_str(&schema_json_str).expect("parsed json");
    assert_eq!(parsed["title"], "controller-app");

    // Check that Schema subcommand was generated in ControllerAppCommands
    let cmd = <ControllerApp as Facet>::command();
    let subcommands: Vec<_> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    assert!(
        subcommands.contains(&"schema".to_string()),
        "subcommands should contain 'schema': {:?}",
        subcommands
    );
    assert!(
        subcommands.contains(&"ping".to_string()),
        "subcommands should contain 'ping': {:?}",
        subcommands
    );
    assert!(
        subcommands.contains(&"status".to_string()),
        "subcommands should contain 'status': {:?}",
        subcommands
    );

    // Run schema command through the runner
    let app = App::<ControllerApp, ControllerApp, clifacet::Plain, ()>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit = app.run_with(
        ["controller-app", "schema"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(exit, 0);
    assert!(err.is_empty());
    let schema_output = String::from_utf8(out).expect("valid utf8");
    let json_val: Value = serde_json::from_str(&schema_output).expect("valid json");
    assert_eq!(json_val["title"], "controller-app");

    // Also run with --schema
    let mut out2 = Vec::new();
    let mut err2 = Vec::new();
    let exit2 = app.run_with(
        ["controller-app", "--schema"],
        Capabilities::headless(),
        prompter,
        &mut out2,
        &mut err2,
    );

    assert_eq!(exit2, 0);
    assert!(err2.is_empty());
    let schema_output2 = String::from_utf8(out2).expect("valid utf8");
    let json_val2: Value = serde_json::from_str(&schema_output2).expect("valid json");
    assert_eq!(json_val2["title"], "controller-app");
}
