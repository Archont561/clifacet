#![cfg(feature = "async")]
#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use clifacet::r#async::{
    AsyncCommand, AsyncHandler, AsyncPlan, AsyncPlanned, AsyncPlannedWith, RunCx,
};
use clifacet::harness::{AsyncRecording, Outcome, Parity, Subject};
use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Failure, Globals, app, commands};

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

// 1. Standalone AsyncPlanned command
#[derive(Clone)]
struct BackupService {
    service_name: String,
    work_executed: Arc<AtomicBool>,
}

impl AsyncPlanned for BackupService {
    type Input = String;
    type Output = String;

    async fn plan(&self, target: Self::Input) -> Result<AsyncPlan<'_, Self::Output>, Failure> {
        let flag = Arc::clone(&self.work_executed);
        let s_name = self.service_name.clone();
        let target_clone = target.clone();
        let plan = AsyncPlan::new(move || async move {
            flag.store(true, Ordering::SeqCst);
            Ok(format!("backup completed for {s_name} -> {target_clone}"))
        })
        .step(format!("prepare backup snapshot for {}", self.service_name))
        .step(format!("upload snapshot to {target}"));

        Ok(plan)
    }
}

// 2. AsyncPlannedWith command with application context
struct CloudStore {
    region: String,
    executed: Arc<AtomicBool>,
}

struct SyncDatabase;

#[clifacet::planned]
impl AsyncPlannedWith<CloudStore> for SyncDatabase {
    type Input = String;
    type Output = String;

    async fn plan<'a>(
        &'a self,
        table: Self::Input,
        store: &'a CloudStore,
    ) -> Result<AsyncPlan<'a, Self::Output>, Failure> {
        let flag = Arc::clone(&store.executed);
        let region = store.region.clone();
        let table_clone = table.clone();
        let plan = AsyncPlan::new(move || async move {
            flag.store(true, Ordering::SeqCst);
            Ok(format!("synced {table_clone} in region {region}"))
        })
        .step(format!("lock table {table} in {}", store.region))
        .step(format!("stream mutations for {table}"))
        .step(format!("release lock on {table}"));

        Ok(plan)
    }
}

// 3. Controller with declarative async commands returning AsyncPlan
static DRY_RUN_WORK_EXECUTED: AtomicBool = AtomicBool::new(false);
static JSON_WORK_EXECUTED: AtomicBool = AtomicBool::new(false);
static REAL_WORK_EXECUTED: AtomicBool = AtomicBool::new(false);

#[app(name = "planapp", commands)]
struct PlanControllerApp {
    #[command(flatten)]
    globals: Globals,
}

#[commands]
impl PlanControllerApp {
    /// Deploy service with async dry-run plan
    pub async fn deploy(&self, service: String) -> Result<AsyncPlan<'static, String>, Failure> {
        let s = service.clone();
        let plan = AsyncPlan::new(move || async move {
            match s.as_str() {
                "auth" => DRY_RUN_WORK_EXECUTED.store(true, Ordering::SeqCst),
                "payment" => JSON_WORK_EXECUTED.store(true, Ordering::SeqCst),
                _ => REAL_WORK_EXECUTED.store(true, Ordering::SeqCst),
            }
            Ok(format!("deployed {s}"))
        })
        .step(format!("pull image for {service}"))
        .step(format!("start container for {service}"))
        .step(format!("verify healthcheck for {service}"));

        Ok(plan)
    }
}

#[tokio::test]
async fn test_async_plan_builder_and_mapping() {
    let executed = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&executed);

    let plan = AsyncPlan::new(move || async move {
        flag.store(true, Ordering::SeqCst);
        Ok(42)
    })
    .step("step 1: compute")
    .steps(["step 2: verify", "step 3: finalize"]);

    assert_eq!(plan.len(), 3);
    assert!(!plan.is_empty());
    assert_eq!(
        plan.steps_slice(),
        &["step 1: compute", "step 2: verify", "step 3: finalize"]
    );
    assert_eq!(
        plan.describe(),
        "step 1: compute\nstep 2: verify\nstep 3: finalize"
    );

    // Ensure mapping transforms output without changing steps
    let mapped = plan.map(|n| n * 2);
    assert_eq!(mapped.len(), 3);
    assert_eq!(
        mapped.describe(),
        "step 1: compute\nstep 2: verify\nstep 3: finalize"
    );

    // Verify work is not run until execute() is awaited
    assert!(!executed.load(Ordering::SeqCst));
    let val = mapped.execute().await.unwrap();
    assert_eq!(val, 84);
    assert!(executed.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_async_planned_command_preview_vs_execute() {
    let executed = Arc::new(AtomicBool::new(false));
    let cmd = BackupService {
        service_name: "postgres".to_string(),
        work_executed: Arc::clone(&executed),
    };

    // Preview generates steps without executing work
    let plan = AsyncCommand::preview(&cmd, "s3://bucket/backup".to_string())
        .await
        .unwrap();
    assert_eq!(
        plan.steps_slice(),
        &[
            "prepare backup snapshot for postgres",
            "upload snapshot to s3://bucket/backup"
        ]
    );
    assert!(!executed.load(Ordering::SeqCst));

    // Execution performs work and marks flag
    let run_cx = RunCx::empty();
    let result = cmd
        .execute("s3://bucket/backup".to_string(), &run_cx)
        .await
        .unwrap();
    assert_eq!(
        result,
        "backup completed for postgres -> s3://bucket/backup"
    );
    assert!(executed.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_async_planned_with_context_and_macro() {
    let executed = Arc::new(AtomicBool::new(false));
    let store = CloudStore {
        region: "us-east-1".to_string(),
        executed: Arc::clone(&executed),
    };
    let cmd = SyncDatabase;

    // Test preview via AsyncHandler trait
    let plan = AsyncHandler::preview(&cmd, "users".to_string(), &store)
        .await
        .unwrap();
    assert_eq!(
        plan.steps_slice(),
        &[
            "lock table users in us-east-1",
            "stream mutations for users",
            "release lock on users"
        ]
    );
    assert!(!executed.load(Ordering::SeqCst));

    // Test handle via AsyncHandler trait
    let run = RunCx::empty();
    let res = AsyncHandler::handle(&cmd, "users".to_string(), &store, &run)
        .await
        .unwrap();
    assert_eq!(res, "synced users in region us-east-1");
    assert!(executed.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_controller_dry_run_does_not_poll_work_future() {
    DRY_RUN_WORK_EXECUTED.store(false, Ordering::SeqCst);
    let app = App::<PlanControllerApp, PlanControllerApp, clifacet::Plain, ()>::for_cli();

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            ["planapp", "deploy", "auth", "--dry-run"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 0);
    assert!(
        !DRY_RUN_WORK_EXECUTED.load(Ordering::SeqCst),
        "work future must NOT be polled on --dry-run"
    );

    let out_str = String::from_utf8(out).unwrap();
    assert!(out_str.contains("pull image for auth"));
    assert!(out_str.contains("start container for auth"));
    assert!(out_str.contains("verify healthcheck for auth"));
}

#[tokio::test]
async fn test_controller_dry_run_json_output() {
    JSON_WORK_EXECUTED.store(false, Ordering::SeqCst);
    let app = App::<PlanControllerApp, PlanControllerApp, clifacet::Plain, ()>::for_cli();

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            ["planapp", "deploy", "payment", "--dry-run", "--json"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 0);
    assert!(
        !JSON_WORK_EXECUTED.load(Ordering::SeqCst),
        "work future must NOT be polled on --dry-run --json"
    );

    let out_str = String::from_utf8(out).unwrap();
    assert!(out_str.contains("\"dry_run\": true"));
    assert!(out_str.contains("\"pull image for payment\""));
}

#[tokio::test]
async fn test_controller_real_execution_performs_work() {
    REAL_WORK_EXECUTED.store(false, Ordering::SeqCst);
    let app = App::<PlanControllerApp, PlanControllerApp, clifacet::Plain, ()>::for_cli();

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app
        .run_with_async(
            ["planapp", "deploy", "billing"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 0);
    assert!(
        REAL_WORK_EXECUTED.load(Ordering::SeqCst),
        "work future MUST be executed when not dry-run"
    );

    let out_str = String::from_utf8(out).unwrap();
    assert_eq!(out_str.trim(), "deployed billing");
}

#[tokio::test]
async fn test_parity_harness_with_async_recording_and_measured_async() {
    let executed = Arc::new(AtomicBool::new(false));
    let cmd = BackupService {
        service_name: "redis".to_string(),
        work_executed: Arc::clone(&executed),
    };

    let (recording, effects) = AsyncRecording::new(cmd, |input| format!("backup target: {input}"));

    // Preview
    let plan = AsyncCommand::preview(&recording, "cold-storage".to_string())
        .await
        .unwrap();
    let preview_steps = plan.steps_slice().to_vec();
    drop(plan);
    assert_eq!(preview_steps.len(), 2);
    assert!(
        effects.is_empty(),
        "preview must not record executed effects"
    );

    // Measured async run
    let run_cx = RunCx::empty();
    let outcome = Outcome::measured_async(|out, _err| {
        Box::pin(async move {
            use std::io::Write as _;
            let res = recording
                .execute("cold-storage".to_string(), &run_cx)
                .await
                .unwrap();
            writeln!(out, "{res}").unwrap();
            0
        })
    })
    .await
    .with_effects(effects.recorded());

    assert_eq!(outcome.exit(), 0);
    assert_eq!(outcome.effects(), &["backup target: cold-storage"]);
    assert_eq!(
        outcome.data().trim(),
        "backup completed for redis -> cold-storage"
    );

    // Parity verification against expected reference outcome
    let expected = Outcome::measured(|out, _err| {
        use std::io::Write as _;
        writeln!(out, "backup completed for redis -> cold-storage").unwrap();
        0
    })
    .with_effects(["backup target: cold-storage"]);

    Parity::new("measured_async", outcome)
        .comparing([Subject::Exit, Subject::Data, Subject::Effects])
        .compare("expected", expected)
        .assert_parity();
}
