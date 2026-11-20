//! Integration tests for structured JSON execution mode and failure schema (CLI-72, decision-25).
#![cfg(feature = "serde")]
#![allow(clippy::pedantic, clippy::assert_is_empty)]

use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Command, ErrorPayload, Failure, Globals, Json, JsonRenderer, Structured};
use serde::{Deserialize, Serialize};

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ServiceStatus {
    service: String,
    active: bool,
    replica_count: usize,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Service {} is {} (replicas: {})",
            self.service,
            if self.active { "active" } else { "inactive" },
            self.replica_count
        )
    }
}

// --- Traditional Command with Command trait ------------------------------------------

#[clifacet::command(input = StatusInput)]
#[derive(Clone)]
#[command(name = "status-app")]
struct StatusApp {
    #[clifacet(long)]
    service: Option<String>,

    #[clifacet(long)]
    fail: bool,

    #[command(flatten)]
    #[clifacet(skip)]
    globals: Globals,
}

#[derive(Clone)]
struct StatusInput {
    service: Option<String>,
    fail: bool,
}

impl Command for StatusApp {
    type Input = StatusInput;
    type Output = String;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        if input.fail {
            return Err(Failure::execution("database connection timed out"));
        }
        let Some(service) = input.service else {
            return Err(Failure::validation(
                "Required argument <service> is missing",
            ));
        };
        let status = ServiceStatus {
            service: service.clone(),
            active: true,
            replica_count: 3,
        };
        if self.globals.json {
            Ok(serde_json::to_string_pretty(&status).unwrap())
        } else {
            Ok(status.to_string())
        }
    }
}

#[test]
fn test_json_flag_formats_successful_output_as_json_on_stdout() {
    let app = App::<StatusApp>::new();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["status-app", "--service", "auth", "--json"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty(), "stderr must be empty on success: {err:?}");

    let out_str = String::from_utf8(out).expect("utf-8 stdout");
    let status: ServiceStatus = serde_json::from_str(&out_str).expect("deserializable stdout");
    assert_eq!(
        status,
        ServiceStatus {
            service: "auth".into(),
            active: true,
            replica_count: 3,
        }
    );
}

#[test]
fn test_plain_output_when_json_flag_omitted() {
    let app = App::<StatusApp>::new();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["status-app", "--service", "auth"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty(), "stderr must be empty on success");

    let out_str = String::from_utf8(out).expect("utf-8 stdout");
    assert_eq!(out_str, "Service auth is active (replicas: 3)\n");
}

#[test]
fn test_execution_failure_formats_as_standardized_json_on_stderr() {
    let app = App::<StatusApp>::new();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["status-app", "--service", "auth", "--fail", "--json"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 1);
    assert!(out.is_empty(), "stdout must be empty on failure: {out:?}");

    let err_str = String::from_utf8(err).expect("utf-8 stderr");
    let payload: ErrorPayload =
        serde_json::from_str(&err_str).expect("deserializable error payload");
    assert_eq!(payload.status, "error");
    assert_eq!(payload.error.kind, "execution");
    assert_eq!(payload.error.code, 1);
    assert_eq!(payload.error.message, "database connection timed out");
}

#[test]
fn test_validation_failure_formats_as_standardized_json_on_stderr() {
    let app = App::<StatusApp>::new();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["status-app", "--json"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 2);
    assert!(
        out.is_empty(),
        "stdout must be empty on validation failure: {out:?}"
    );

    let err_str = String::from_utf8(err).expect("utf-8 stderr");
    let payload: ErrorPayload =
        serde_json::from_str(&err_str).expect("deserializable error payload");
    assert_eq!(payload.status, "error");
    assert_eq!(payload.error.kind, "validation");
    assert_eq!(payload.error.code, 2);
    assert_eq!(
        payload.error.message,
        "Required argument <service> is missing"
    );
}

#[test]
fn test_clap_usage_error_with_json_flag_formats_as_standardized_json_on_stderr() {
    let app = App::<StatusApp>::new();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["status-app", "--json", "--nonexistent-flag"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 2);
    assert!(out.is_empty(), "stdout must be empty on clap usage error");

    let err_str = String::from_utf8(err).expect("utf-8 stderr");
    let payload: ErrorPayload =
        serde_json::from_str(&err_str).expect("deserializable error payload");
    assert_eq!(payload.status, "error");
    assert_eq!(payload.error.code, 2);
    assert!(
        payload.error.message.contains("--nonexistent-flag"),
        "error message should cite the unknown flag: {}",
        payload.error.message
    );
}

// --- Declarative Controller with #[clifacet::app] and #[clifacet::commands] -----------

#[clifacet::app(name = "fleet", commands)]
struct FleetApp {
    #[command(flatten)]
    globals: Globals,
}

#[clifacet::commands]
impl FleetApp {
    /// Returns service status as a structured JSON outcome payload.
    fn inspect(&self, service: String) -> Result<Json<ServiceStatus>, Failure> {
        if service == "fail" {
            return Err(Failure::execution("host unreachable"));
        }
        Ok(Json(ServiceStatus {
            service,
            active: true,
            replica_count: 5,
        }))
    }

    /// Returns structured outcome with human display or JSON.
    fn dual(&self, service: String) -> Result<Structured<ServiceStatus>, Failure> {
        let status = ServiceStatus {
            service: service.clone(),
            active: true,
            replica_count: 2,
        };
        Ok(Structured::new(
            status,
            format!("Service {service} is ready"),
        ))
    }
}

#[test]
fn test_controller_json_command_execution() {
    let app = App::<FleetApp>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["fleet", "--json", "inspect", "billing"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty(), "stderr must be empty");

    let out_str = String::from_utf8(out).expect("utf-8 stdout");
    let status: ServiceStatus = serde_json::from_str(&out_str).expect("deserializable JSON");
    assert_eq!(status.service, "billing");
    assert_eq!(status.replica_count, 5);
}

#[test]
fn test_controller_dual_structured_without_json() {
    let app = App::<FleetApp>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["fleet", "dual", "payments"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty());
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Service payments is ready\n"
    );
}

#[test]
fn test_controller_dual_structured_with_json() {
    let app = App::<FleetApp>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["fleet", "--json", "dual", "payments"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty(), "stderr must be empty");

    let out_str = String::from_utf8(out).expect("utf-8 stdout");
    let status: ServiceStatus = serde_json::from_str(&out_str).expect("deserializable JSON");
    assert_eq!(status.service, "payments");
    assert_eq!(status.replica_count, 2);
}

#[test]
fn test_controller_error_in_json_mode() {
    let app = App::<FleetApp>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["fleet", "--json", "inspect", "fail"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 1);
    assert!(out.is_empty(), "stdout must be empty");

    let err_str = String::from_utf8(err).expect("utf-8 stderr");
    let payload: ErrorPayload =
        serde_json::from_str(&err_str).expect("deserializable error payload");
    assert_eq!(payload.status, "error");
    assert_eq!(payload.error.kind, "execution");
    assert_eq!(payload.error.code, 1);
    assert_eq!(payload.error.message, "host unreachable");
}

#[clifacet::command(input = CustomInput)]
#[derive(Clone)]
#[command(name = "custom-app")]
struct CustomTypeApp {
    #[clifacet(long)]
    service: Option<String>,
}

#[derive(Clone)]
struct CustomInput {
    service: Option<String>,
}

impl Command for CustomTypeApp {
    type Input = CustomInput;
    type Output = ServiceStatus;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        Ok(ServiceStatus {
            service: input.service.unwrap_or_default(),
            active: true,
            replica_count: 5,
        })
    }
}

#[test]
fn test_json_renderer_explicitly_configured() {
    let app = App::<CustomTypeApp>::new().render(JsonRenderer::default());
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["custom-app", "--service", "database"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty());

    let out_str = String::from_utf8(out).expect("utf-8 stdout");
    let status: ServiceStatus = serde_json::from_str(&out_str).expect("deserializable stdout");
    assert_eq!(status.service, "database");
    assert_eq!(status.replica_count, 5);
}

// --- Dry run with --json --------------------------------------------------------------

#[clifacet::command(input = PlannedInput)]
#[derive(Clone)]
#[command(name = "plan-app")]
struct PlannedApp {
    #[command(flatten)]
    #[clifacet(skip)]
    globals: Globals,
}

#[derive(Clone)]
struct PlannedInput;

impl clifacet::command::Planned for PlannedApp {
    type Input = PlannedInput;
    type Output = String;

    fn plan(
        &self,
        _input: Self::Input,
    ) -> Result<clifacet::command::Plan<'_, Self::Output>, Failure> {
        let plan = clifacet::command::Plan::new(|| Ok("done".into()))
            .step("provision instance")
            .step("attach volume");
        Ok(plan)
    }
}

#[test]
fn test_dry_run_with_json_outputs_structured_plan_on_stdout() {
    let app = App::<PlannedApp>::new();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["plan-app", "--dry-run", "--json"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty(), "stderr must be empty on dry-run");

    let out_str = String::from_utf8(out).expect("utf-8 stdout");
    let val: serde_json::Value = serde_json::from_str(&out_str).expect("valid JSON dry-run output");
    assert_eq!(val["dry_run"], true);
    assert_eq!(
        val["steps"],
        serde_json::json!(["provision instance", "attach volume"])
    );
}

#[test]
fn test_non_interactive_piped_separation_of_data_and_logs() {
    let app = App::<StatusApp>::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    // Headless / non-interactive (piped) mode: success
    let code = app.run_with(
        ["status-app", "--service", "payments", "--json"],
        Capabilities::headless(),
        prompter,
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(code, 0);
    assert!(
        stderr.is_empty(),
        "stderr must be strictly empty on successful JSON run"
    );
    let out_str = String::from_utf8(stdout).expect("utf-8 stdout");
    let val: serde_json::Value = serde_json::from_str(&out_str).expect("valid JSON stdout");
    assert_eq!(val["service"], "payments");

    // Headless / non-interactive (piped) mode: failure
    let mut err_stdout = Vec::new();
    let mut err_stderr = Vec::new();
    let fail_code = app.run_with(
        ["status-app", "--service", "--fail", "--json"],
        Capabilities::headless(),
        prompter,
        &mut err_stdout,
        &mut err_stderr,
    );

    assert_ne!(fail_code, 0);
    assert!(
        err_stdout.is_empty(),
        "stdout must be strictly empty on JSON error"
    );
    let err_str = String::from_utf8(err_stderr).expect("utf-8 stderr");
    let err_val: ErrorPayload =
        serde_json::from_str(&err_str).expect("valid error payload on stderr");
    assert_eq!(err_val.status, "error");
}
