#![cfg(all(feature = "async", feature = "tokio"))]
#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use std::time::Duration;

use clifacet::r#async::RunCx;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Failure, Globals, app, commands};

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Clone, Default)]
struct CancelTestCx;

#[app(
    name = "canceldemo",
    about = "Cancellation and timeout contract demo",
    commands
)]
struct CancelApp {
    #[command(flatten)]
    globals: Globals,
    #[clifacet(context = build_context)]
    cx: CancelTestCx,
}

fn build_context(_cli: &CancelApp) -> Result<CancelTestCx, Failure> {
    Ok(CancelTestCx)
}

#[commands]
impl CancelApp {
    /// Command that gracefully cleans up when cancelled.
    pub async fn download(&self, filename: String, run: &RunCx) -> Result<String, Failure> {
        run.request_interrupt(); // simulate first Ctrl-C signal
        if run.is_cancelled() {
            return Err(Failure::cancelled(format!(
                "partial download saved to {filename}.part"
            )));
        }
        Ok(format!("downloaded {filename}"))
    }

    /// Command that ignores cancellation and hangs.
    pub async fn stubborn(&self, run: &RunCx) -> Result<String, Failure> {
        run.request_interrupt(); // first Ctrl-C
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok("completed".to_string())
    }

    /// Command that triggers a second Ctrl-C immediately.
    pub async fn double_interrupt(&self, run: &RunCx) -> Result<String, Failure> {
        run.request_interrupt(); // first Ctrl-C
        run.request_interrupt(); // second Ctrl-C -> hard abort
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok("completed".to_string())
    }

    /// Slow command for timeout testing.
    pub async fn slow(&self, millis: u64) -> Result<String, Failure> {
        tokio::time::sleep(Duration::from_millis(millis)).await;
        Ok(format!("slow finished in {millis}ms"))
    }
}

#[tokio::test(start_paused = true)]
async fn test_cooperative_cancellation_maps_to_exit_130_and_preserves_message() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "download", "largefile.iso"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 130);
    let err_str = String::from_utf8(err).unwrap();
    assert!(
        err_str.contains("canceldemo: partial download saved to largefile.iso.part"),
        "Unexpected error output: {err_str}"
    );
}

#[tokio::test(start_paused = true)]
async fn test_cooperative_cancellation_json_mode() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "--json", "download", "largefile.iso"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 130);
    let err_str = String::from_utf8(err).unwrap();
    assert!(err_str.contains("\"kind\": \"cancelled\""), "{err_str}");
    assert!(err_str.contains("\"code\": 130"), "{err_str}");
    assert!(
        err_str.contains("partial download saved to largefile.iso.part"),
        "{err_str}"
    );
}

#[tokio::test(start_paused = true)]
async fn test_second_ctrl_c_immediately_aborts_active_task() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "double-interrupt"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 130);
    let err_str = String::from_utf8(err).unwrap();
    assert!(
        err_str.contains("canceldemo: operation aborted by interrupt"),
        "Unexpected error output: {err_str}"
    );
}

#[tokio::test(start_paused = true)]
async fn test_second_ctrl_c_json_mode() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "--json", "double-interrupt"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 130);
    let err_str = String::from_utf8(err).unwrap();
    assert!(err_str.contains("\"kind\": \"cancelled\""), "{err_str}");
    assert!(err_str.contains("\"code\": 130"), "{err_str}");
    assert!(
        err_str.contains("operation aborted by interrupt"),
        "{err_str}"
    );
}

#[tokio::test(start_paused = true)]
async fn test_grace_period_expiration_hard_aborts() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli()
        .grace_period(Duration::from_millis(50));
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "stubborn"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 130);
    let err_str = String::from_utf8(err).unwrap();
    assert!(
        err_str.contains("canceldemo: operation aborted by interrupt"),
        "Unexpected error output: {err_str}"
    );
}

#[tokio::test(start_paused = true)]
async fn test_timeout_flag_aborts_slow_execution() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "--timeout", "20ms", "slow", "200"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_ne!(code, 0);
    assert_ne!(code, 130);
    let err_str = String::from_utf8(err).unwrap();
    assert!(
        err_str.contains("canceldemo: timed out after 20ms"),
        "Unexpected error output: {err_str}"
    );
}

#[tokio::test(start_paused = true)]
async fn test_timeout_flag_json_mode() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "--json", "--timeout", "20ms", "slow", "200"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_ne!(code, 0);
    let err_str = String::from_utf8(err).unwrap();
    assert!(err_str.contains("\"kind\": \"execution\""), "{err_str}");
    assert!(err_str.contains("\"code\": 1"), "{err_str}");
    assert!(err_str.contains("timed out after 20ms"), "{err_str}");
}

#[tokio::test(start_paused = true)]
async fn test_timeout_flag_completes_if_within_deadline() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "--timeout", "100ms", "slow", "10"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 0);
    let out_str = String::from_utf8(out).unwrap();
    assert_eq!(out_str, "slow finished in 10ms\n");
}

#[tokio::test(start_paused = true)]
async fn test_invalid_timeout_syntax_fails_validation() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "--timeout", "not_a_duration", "slow", "10"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 2);
    let err_str = String::from_utf8(err).unwrap();
    assert!(
        err_str.contains("invalid duration 'not_a_duration'"),
        "Unexpected error output: {err_str}"
    );
}

#[tokio::test(start_paused = true)]
async fn test_dry_run_never_polls_work_future() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "--dry-run", "slow", "100"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    // A command without a plan is refused for dry-run before polling
    assert_eq!(code, 2);
    let err_str = String::from_utf8(err).unwrap();
    assert!(err_str.contains("cannot be run with --dry-run"));
}

#[tokio::test(start_paused = true)]
async fn test_concurrent_background_task_interrupts_running_command() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli()
        .grace_period(Duration::from_millis(50))
        .async_runner(|_, _, run| {
            let cancel = run.cancel().clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                cancel.interrupt(); // 1st Ctrl-C
            });
            async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok("unreachable".to_string())
            }
        });

    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "slow", "100"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 130);
    let err_str = String::from_utf8(err).unwrap();
    assert!(err_str.contains("canceldemo: operation aborted by interrupt"));
}

#[tokio::test(start_paused = true)]
async fn test_help_never_times_out() {
    let app = App::<CancelApp, CancelApp, clifacet::Plain, CancelTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["canceldemo", "--timeout", "1ms", "--help"],
            Capabilities::headless(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 0);
    let out_str = String::from_utf8(out).unwrap();
    assert!(out_str.contains("Usage: canceldemo"));
}
