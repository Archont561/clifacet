//! Tests for async choice providers and dynamic shell completions integration (CLI-85, AC #3 & #4).

#![cfg(all(feature = "completions", feature = "async"))]
#![allow(
    clippy::pedantic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use std::sync::{Arc, Mutex};

use clifacet::policy::{Capabilities, Mode, Policy, Request};
use clifacet::prompt::{Answer, Choice, Prompter, Scripted, Session};
use clifacet::{App, Failure, app, commands};

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Clone, Debug)]
struct FleetDatabase {
    services: Arc<Mutex<Vec<String>>>,
    environments: Arc<Mutex<Vec<String>>>,
    regions: Arc<Mutex<Vec<String>>>,
    tags: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone)]
struct FleetContext {
    db: FleetDatabase,
}

static ASYNC_CHOICES_EXTRA_SERVICES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn make_test_context() -> FleetContext {
    let mut initial_services = vec![
        "auth-service".to_string(),
        "billing-service".to_string(),
        "web-gateway".to_string(),
    ];
    if let Ok(guard) = ASYNC_CHOICES_EXTRA_SERVICES.lock() {
        initial_services.extend(guard.clone());
    }

    FleetContext {
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
            tags: Arc::new(Mutex::new(vec![
                "critical".to_string(),
                "canary".to_string(),
                "experiment".to_string(),
            ])),
        },
    }
}

fn build_context(_cli: &FleetApp) -> Result<FleetContext, Failure> {
    Ok(make_test_context())
}

struct YieldOnce(bool);

impl std::future::Future for YieldOnce {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        if self.0 {
            std::task::Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    }
}

async fn yield_once() {
    YieldOnce(false).await;
}

/// Async choice provider returning `Vec<String>`, accessing `&Cx` and `partial`.
async fn async_complete_services(cx: &FleetContext, partial: &str) -> Vec<String> {
    yield_once().await;

    cx.db
        .services
        .lock()
        .unwrap()
        .iter()
        .filter(|s| s.starts_with(partial))
        .cloned()
        .collect()
}

/// Async choice provider returning `Vec<Choice>`, accessing `&Cx` and `partial`.
async fn async_complete_environments(cx: &FleetContext, partial: &str) -> Vec<Choice> {
    yield_once().await;

    cx.db
        .environments
        .lock()
        .unwrap()
        .iter()
        .filter(|e| e.starts_with(partial))
        .map(|e| Choice::new(e.clone()).with_help(format!("Target environment: {e}")))
        .collect()
}

/// Async choice provider accessing `&Cx` and `partial`.
async fn async_complete_regions(cx: &FleetContext, partial: &str) -> Vec<String> {
    yield_once().await;

    cx.db
        .regions
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.starts_with(partial))
        .cloned()
        .collect()
}

/// Synchronous choice provider using `choices = ...`.
fn sync_complete_tags(cx: &FleetContext, partial: &str) -> Vec<String> {
    cx.db
        .tags
        .lock()
        .unwrap()
        .iter()
        .filter(|t| t.starts_with(partial))
        .cloned()
        .collect()
}

#[app(
    name = "fleet",
    version = "1.2.0",
    about = "Fleet deployment CLI with async choice providers",
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
    /// Deploy service to environment with async choices.
    pub async fn deploy(
        &self,
        #[clifacet(choices = async_complete_services)] service: String,
        #[clifacet(long, choices = async_complete_environments)] env: String,
        #[clifacet(long, choices = async_complete_regions)] region: Option<String>,
        #[clifacet(long, choices = sync_complete_tags)] tag: Option<String>,
    ) -> Result<String, Failure> {
        let reg = region.unwrap_or_else(|| "default".into());
        let t = tag.unwrap_or_else(|| "none".into());
        Ok(format!("deployed {service} to {env} ({reg}) [tag: {t}]"))
    }

    /// Check status of fleet.
    pub fn status(&self) -> Result<String, Failure> {
        Ok("all systems operational".into())
    }
}

#[tokio::test]
async fn test_async_choice_provider_accessing_context_and_partial() {
    let _guard = TEST_MUTEX.lock().await;
    ASYNC_CHOICES_EXTRA_SERVICES.lock().unwrap().clear();
    let cx = make_test_context();

    // Complete positional service argument with prefix "bil"
    let completions = FleetApp::complete_argument(&cx, "deploy", "service", "bil");
    assert_eq!(completions, Some(vec!["billing-service".to_string()]));

    // Complete positional service argument with empty prefix
    let completions = FleetApp::complete_argument(&cx, "deploy", "service", "");
    assert_eq!(
        completions,
        Some(vec![
            "auth-service".to_string(),
            "billing-service".to_string(),
            "web-gateway".to_string(),
        ])
    );
}

#[tokio::test]
async fn test_async_choice_provider_returning_choices_and_sync_choices() {
    let _guard = TEST_MUTEX.lock().await;
    ASYNC_CHOICES_EXTRA_SERVICES.lock().unwrap().clear();
    let cx = make_test_context();

    // Complete --env flag (returns Choice which displays as label)
    let env_completions = FleetApp::complete_argument(&cx, "deploy", "--env", "st");
    assert_eq!(
        env_completions,
        Some(vec!["staging-us".to_string(), "staging-eu".to_string()])
    );

    // Complete --region flag (async function)
    let region_completions = FleetApp::complete_argument(&cx, "deploy", "--region", "us");
    assert_eq!(
        region_completions,
        Some(vec!["us-east-1".to_string(), "us-west-2".to_string()])
    );

    // Complete --tag flag (sync choice provider with choices = ...)
    let tag_completions = FleetApp::complete_argument(&cx, "deploy", "--tag", "ca");
    assert_eq!(tag_completions, Some(vec!["canary".to_string()]));
}

#[tokio::test]
async fn test_dynamic_shell_completion_integration_with_async_choices() {
    let _guard = TEST_MUTEX.lock().await;
    ASYNC_CHOICES_EXTRA_SERVICES.lock().unwrap().clear();
    let app = App::<FleetApp, FleetApp, clifacet::Plain, FleetContext>::for_cli();

    // 1. Dynamic positional completion: services
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            ["fleet", "completions", "--complete", "--", "deploy", ""],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;
    assert_eq!(code, 0, "STDERR was: {}", String::from_utf8_lossy(&err));
    let items = String::from_utf8(out).unwrap();
    assert!(items.contains("auth-service"));
    assert!(items.contains("billing-service"));
    assert!(items.contains("web-gateway"));

    // 2. Filtered positional completion
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            ["fleet", "completions", "--complete", "--", "deploy", "auth"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;
    assert_eq!(code, 0);
    let items = String::from_utf8(out).unwrap();
    assert_eq!(items.trim(), "auth-service");

    // 3. Dynamic flag completion with async Choice return type: --env
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
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
        )
        .await;
    assert_eq!(code, 0);
    let items = String::from_utf8(out).unwrap();
    assert!(items.contains("staging-us"));
    assert!(items.contains("staging-eu"));
    assert!(!items.contains("production"));

    // 4. Dynamic flag completion with async function: --region
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            [
                "fleet",
                "completions",
                "--complete",
                "--",
                "deploy",
                "--region",
                "us",
            ],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;
    assert_eq!(code, 0);
    let items = String::from_utf8(out).unwrap();
    assert!(items.contains("us-east-1"));
    assert!(items.contains("us-west-2"));

    // 5. Dynamic flag completion with sync choices: --tag
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            [
                "fleet",
                "completions",
                "--complete",
                "--",
                "deploy",
                "--tag",
                "can",
            ],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;
    assert_eq!(code, 0);
    let items = String::from_utf8(out).unwrap();
    assert_eq!(items.trim(), "canary");
}

#[tokio::test]
async fn test_dynamic_completion_reflects_runtime_context_mutations_async() {
    let _guard = TEST_MUTEX.lock().await;
    ASYNC_CHOICES_EXTRA_SERVICES.lock().unwrap().clear();
    let app = App::<FleetApp, FleetApp, clifacet::Plain, FleetContext>::for_cli();

    // Verify initial services
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            [
                "fleet",
                "completions",
                "--complete",
                "--",
                "deploy",
                "telemetry",
            ],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;
    assert_eq!(
        code,
        0,
        "first query STDERR: {}",
        String::from_utf8_lossy(&err)
    );
    assert_eq!(String::from_utf8(out).unwrap().trim(), "");

    // Add new service to global test extra services
    ASYNC_CHOICES_EXTRA_SERVICES
        .lock()
        .unwrap()
        .push("telemetry-collector".to_string());

    // Second completion query: live context should reflect the newly added service
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            [
                "fleet",
                "completions",
                "--complete",
                "--",
                "deploy",
                "telemetry",
            ],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;
    assert_eq!(
        code,
        0,
        "second query STDERR: {}",
        String::from_utf8_lossy(&err)
    );
    let items = String::from_utf8(out).unwrap();
    assert_eq!(items.trim(), "telemetry-collector");

    ASYNC_CHOICES_EXTRA_SERVICES.lock().unwrap().clear();
}

#[tokio::test]
async fn test_session_choose_async_integration() {
    let plan = Policy::up_to(Mode::Interactive)
        .resolve(Request::none(), Capabilities::terminal())
        .expect("prompts permitted");
    let mut session = Session::start(plan, || {
        Box::new(Scripted::new([Answer::Text("staging".to_string())]))
    });

    let chosen = session
        .choose_async(None, "Which environment to deploy to?", || async {
            vec!["dev", "staging", "prod"]
        })
        .await
        .unwrap();

    assert_eq!(chosen, Some("staging"));

    // If value already supplied, choose_async does not prompt
    let supplied = session
        .choose_async(Some("pre-chosen"), "Question?", || async {
            panic!("should not be called when supplied is Some");
        })
        .await
        .unwrap();

    assert_eq!(supplied, Some("pre-chosen"));
}
