#![cfg(feature = "completions")]
#![allow(
    clippy::pedantic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use clifacet::completions::Shell;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Failure, app, commands};
use std::sync::{Arc, Mutex};

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Clone, Debug)]
struct FleetDatabase {
    services: Arc<Mutex<Vec<String>>>,
    environments: Arc<Mutex<Vec<String>>>,
    regions: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone)]
struct FleetContext {
    db: FleetDatabase,
}

static TEST_EXTRA_SERVICES: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn build_context(_cli: &FleetApp) -> Result<FleetContext, Failure> {
    let mut initial_services = vec![
        "auth-service".to_string(),
        "billing-service".to_string(),
        "web-gateway".to_string(),
    ];
    if let Ok(guard) = TEST_EXTRA_SERVICES.lock() {
        initial_services.extend(guard.clone());
    }

    Ok(FleetContext {
        db: FleetDatabase {
            services: Arc::new(Mutex::new(initial_services)),
            environments: Arc::new(Mutex::new(vec![
                "development".to_string(),
                "staging-us".to_string(),
                "staging-eu".to_string(),
                "production".to_string(),
            ])),
            regions: Arc::new(Mutex::new(vec![
                "us-east-1".to_string(),
                "us-west-2".to_string(),
                "eu-central-1".to_string(),
            ])),
        },
    })
}

fn complete_services(cx: &FleetContext, current: &str) -> Vec<String> {
    cx.db
        .services
        .lock()
        .unwrap()
        .iter()
        .filter(|s| s.starts_with(current))
        .cloned()
        .collect()
}

fn complete_environments(cx: &FleetContext, current: &str) -> Vec<String> {
    cx.db
        .environments
        .lock()
        .unwrap()
        .iter()
        .filter(|e| e.starts_with(current))
        .cloned()
        .collect()
}

fn complete_regions(cx: &FleetContext, current: &str) -> Vec<String> {
    cx.db
        .regions
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.starts_with(current))
        .cloned()
        .collect()
}

#[app(
    name = "fleet",
    version = "1.2.0",
    about = "Fleet deployment CLI",
    commands
)]
struct FleetApp {
    #[clifacet(long)]
    verbose: bool,

    #[clifacet(context = build_context)]
    cx: FleetContext,
}

#[commands]
impl FleetApp {
    /// Deploy service to environment.
    pub fn deploy(
        &self,
        #[clifacet(complete = complete_services)] service: String,
        #[clifacet(long, complete = complete_environments)] env: String,
        #[clifacet(long, complete = complete_regions)] region: Option<String>,
    ) -> Result<String, Failure> {
        let reg = region.unwrap_or_else(|| "default".into());
        Ok(format!("deployed {service} to {env} ({reg})"))
    }

    /// Check status of fleet.
    pub fn status(&self) -> Result<String, Failure> {
        Ok("all systems operational".into())
    }
}

#[test]
fn test_static_completion_script_generation_across_all_shells() {
    let bash = FleetApp::completion_script(Shell::Bash);
    assert!(bash.contains("# Bash completion script for fleet"));
    assert!(bash.contains("_fleet_completions()"));
    assert!(bash.contains(r#""fleet" completions --complete --"#));
    assert!(bash.contains("complete -F _fleet_completions -o default fleet"));

    let zsh = FleetApp::completion_script(Shell::Zsh);
    assert!(zsh.contains("#compdef fleet"));
    assert!(zsh.contains("_fleet()"));
    assert!(zsh.contains("fleet completions --complete --"));
    assert!(zsh.contains("_describe -t commands"));

    let fish = FleetApp::completion_script(Shell::Fish);
    assert!(fish.contains("# Fish completion script for fleet"));
    assert!(fish.contains("__fish_fleet_complete"));
    assert!(fish.contains("complete -c fleet"));

    let pwsh = FleetApp::completion_script(Shell::PowerShell);
    assert!(pwsh.contains("# PowerShell completion script for fleet"));
    assert!(pwsh.contains("Register-ArgumentCompleter"));
    assert!(pwsh.contains("fleet completions --complete --"));
}

#[test]
fn test_cli_subcommand_completions_emits_scripts() {
    let app = App::<FleetApp, FleetApp, clifacet::Plain, FleetContext>::for_cli();

    // 1. Bash completion via CLI
    let mut out_bash = Vec::new();
    let mut err_bash = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "bash"],
        Capabilities::headless(),
        prompter,
        &mut out_bash,
        &mut err_bash,
    );
    assert_eq!(code, 0);
    let bash_str = String::from_utf8(out_bash).unwrap();
    assert!(bash_str.contains("_fleet_completions()"));

    // 2. Zsh completion via CLI
    let mut out_zsh = Vec::new();
    let mut err_zsh = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "zsh"],
        Capabilities::headless(),
        prompter,
        &mut out_zsh,
        &mut err_zsh,
    );
    assert_eq!(code, 0);
    let zsh_str = String::from_utf8(out_zsh).unwrap();
    assert!(zsh_str.contains("#compdef fleet"));

    // 3. Fish completion via CLI
    let mut out_fish = Vec::new();
    let mut err_fish = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "fish"],
        Capabilities::headless(),
        prompter,
        &mut out_fish,
        &mut err_fish,
    );
    assert_eq!(code, 0);
    let fish_str = String::from_utf8(out_fish).unwrap();
    assert!(fish_str.contains("__fish_fleet_complete"));

    // 4. PowerShell completion via CLI
    let mut out_pwsh = Vec::new();
    let mut err_pwsh = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "powershell"],
        Capabilities::headless(),
        prompter,
        &mut out_pwsh,
        &mut err_pwsh,
    );
    assert_eq!(code, 0);
    let pwsh_str = String::from_utf8(out_pwsh).unwrap();
    assert!(pwsh_str.contains("Register-ArgumentCompleter"));

    // 5. Missing shell argument error
    let mut out_err = Vec::new();
    let mut err_err = Vec::new();
    let code = app.run_with(
        ["fleet", "completions"],
        Capabilities::headless(),
        prompter,
        &mut out_err,
        &mut err_err,
    );
    assert_ne!(code, 0);
    assert!(
        String::from_utf8(err_err)
            .unwrap()
            .contains("missing shell argument")
    );

    // 6. Invalid shell argument error
    let mut out_inv = Vec::new();
    let mut err_inv = Vec::new();
    let code_inv = app.run_with(
        ["fleet", "completions", "unknown-shell"],
        Capabilities::headless(),
        prompter,
        &mut out_inv,
        &mut err_inv,
    );
    assert_ne!(code_inv, 0);
    assert!(
        String::from_utf8(err_inv)
            .unwrap()
            .contains("unsupported shell")
    );
}

#[test]
fn test_dynamic_completion_subcommands_and_flags() {
    let app = App::<FleetApp, FleetApp, clifacet::Plain, FleetContext>::for_cli();

    // 1. Complete subcommands at root
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "--complete", "--", ""],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let subcmds = String::from_utf8(out).unwrap();
    assert!(subcmds.contains("deploy"));
    assert!(subcmds.contains("status"));
    assert!(subcmds.contains("completions"));

    // 2. Filter subcommands with prefix
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "--complete", "--", "dep"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let subcmds = String::from_utf8(out).unwrap();
    assert_eq!(subcmds.trim(), "deploy");

    // 3. Complete flags under deploy
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "--complete", "--", "deploy", "--"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let flags = String::from_utf8(out).unwrap();
    assert!(flags.contains("--env"));
    assert!(flags.contains("--region"));
    assert!(flags.contains("--help"));
}

#[test]
fn test_dynamic_completion_positional_and_flag_values_with_live_context() {
    let app = App::<FleetApp, FleetApp, clifacet::Plain, FleetContext>::for_cli();

    // 1. Dynamic positional completion: services
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "--complete", "--", "deploy", ""],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let items = String::from_utf8(out).unwrap();
    assert!(items.contains("auth-service"));
    assert!(items.contains("billing-service"));
    assert!(items.contains("web-gateway"));

    // 2. Filtered positional completion
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "--complete", "--", "deploy", "bil"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let items = String::from_utf8(out).unwrap();
    assert_eq!(items.trim(), "billing-service");

    // 3. Dynamic flag value completion: --env <value>
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        [
            "fleet",
            "completions",
            "--complete",
            "--",
            "deploy",
            "--env",
            "st",
        ],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let items = String::from_utf8(out).unwrap();
    assert!(items.contains("staging-us"));
    assert!(items.contains("staging-eu"));
    assert!(!items.contains("production"));

    // 4. Dynamic flag inline value completion: --region=<value>
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        [
            "fleet",
            "completions",
            "--complete",
            "--",
            "deploy",
            "--region=us",
        ],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let items = String::from_utf8(out).unwrap();
    assert!(items.contains("--region=us-east-1"));
    assert!(items.contains("--region=us-west-2"));
    assert!(!items.contains("--region=eu-central-1"));
}

#[test]
fn test_dynamic_completion_reflects_runtime_context_mutations() {
    let app = App::<FleetApp, FleetApp, clifacet::Plain, FleetContext>::for_cli();

    // Verify initial services don't match "pay"
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["fleet", "completions", "--complete", "--", "deploy", "pay"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap().trim(), "");

    // Mutate mock context state
    TEST_EXTRA_SERVICES
        .lock()
        .unwrap()
        .push("payment-service".to_string());

    // Verify completions now immediately discover the new service
    let mut out2 = Vec::new();
    let mut err2 = Vec::new();
    let code2 = app.run_with(
        ["fleet", "completions", "--complete", "--", "deploy", "pay"],
        Capabilities::headless(),
        prompter,
        &mut out2,
        &mut err2,
    );
    assert_eq!(code2, 0);
    assert_eq!(String::from_utf8(out2).unwrap().trim(), "payment-service");

    // Clean up
    TEST_EXTRA_SERVICES.lock().unwrap().clear();

    // Deploy service execution
    let mut out_dep = Vec::new();
    let mut err_dep = Vec::new();
    let code_dep = app.run_with(
        ["fleet", "deploy", "auth-service", "--env", "production"],
        Capabilities::headless(),
        prompter,
        &mut out_dep,
        &mut err_dep,
    );
    assert_eq!(code_dep, 0);
    assert_eq!(
        String::from_utf8(out_dep).unwrap().trim(),
        "deployed auth-service to production (default)"
    );
}
