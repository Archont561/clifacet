#![cfg(feature = "async")]
#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use clifacet::r#async::RunCx;
use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Failure, app, commands};

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Clone)]
struct AsyncTestCx {
    counter: Arc<AtomicU32>,
    hook_log: Arc<Mutex<Vec<String>>>,
}

static CX_INIT_COUNT: AtomicU32 = AtomicU32::new(0);

async fn build_async_cx(_cli: &AsyncTestCli) -> Result<AsyncTestCx, Failure> {
    CX_INIT_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(AsyncTestCx {
        counter: Arc::new(AtomicU32::new(100)),
        hook_log: Arc::new(Mutex::new(Vec::new())),
    })
}

async fn hook_before_async(
    _cli: &AsyncTestCli,
    cmd: &str,
    cx: &AsyncTestCx,
) -> Result<(), Failure> {
    cx.hook_log
        .lock()
        .unwrap()
        .push(format!("before_async:{cmd}"));
    Ok(())
}

async fn hook_success_async(
    _cli: &AsyncTestCli,
    cmd: &str,
    cx: &AsyncTestCx,
) -> Result<(), Failure> {
    cx.hook_log
        .lock()
        .unwrap()
        .push(format!("success_async:{cmd}"));
    Ok(())
}

async fn hook_failure_async(
    _cli: &AsyncTestCli,
    cmd: &str,
    _err: &Failure,
    cx: Option<&AsyncTestCx>,
) {
    if let Some(cx) = cx {
        cx.hook_log
            .lock()
            .unwrap()
            .push(format!("failure_async:{cmd}"));
    }
}

#[app(
    name = "asynccli",
    version = "1.0.0",
    about = "Async controller app",
    commands,
    before_run = hook_before_async,
    after_success = hook_success_async,
    after_failure = hook_failure_async,
)]
struct AsyncTestCli {
    #[clifacet(long)]
    prefix: Option<String>,

    #[clifacet(context = build_async_cx)]
    cx: AsyncTestCx,
}

#[commands]
impl AsyncTestCli {
    /// Pure async fetch command
    pub async fn fetch(&self, item: String) -> Result<String, Failure> {
        let val = self.cx.counter.fetch_add(1, Ordering::SeqCst);
        let prefix = self.prefix.as_deref().unwrap_or("item");
        Ok(format!("{prefix}:{item}:{val}"))
    }

    /// Async command receiving RunCx
    pub async fn compute(&self, a: u32, run: &RunCx) -> Result<String, Failure> {
        assert!(!run.cancel().is_cancelled());
        run.progress().message("Computing");
        let val = self.cx.counter.fetch_add(a, Ordering::SeqCst);
        Ok(format!("sum:{}", val + a))
    }

    /// Deliberate async failure
    pub async fn bad_cmd(&self) -> Result<(), Failure> {
        Err(Failure::execution("deliberate async failure"))
    }
}

#[derive(Clone)]
struct AsyncHelpCx;

static HELP_CX_INIT_COUNT: AtomicU32 = AtomicU32::new(0);

async fn build_help_cx(_cli: &AsyncHelpCli) -> Result<AsyncHelpCx, Failure> {
    HELP_CX_INIT_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(AsyncHelpCx)
}

#[app(name = "helpcli", commands)]
struct AsyncHelpCli {
    #[clifacet(context = build_help_cx)]
    cx: AsyncHelpCx,
}

#[commands]
impl AsyncHelpCli {
    pub async fn run_task(&self) -> Result<(), Failure> {
        Ok(())
    }
}

#[test]
fn test_async_commands_help_does_not_initialize_context() {
    let app = App::<AsyncHelpCli, AsyncHelpCli, clifacet::Plain, AsyncHelpCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = clifacet::cli::block_on(app.run_with_async(
        ["helpcli", "--help"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    ));
    assert_eq!(code, 0);
    assert_eq!(
        HELP_CX_INIT_COUNT.load(Ordering::SeqCst),
        0,
        "Lazy async context must not be initialized on --help"
    );
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("run-task"));
}

#[test]
fn test_async_commands_dispatch_and_hooks() {
    CX_INIT_COUNT.store(0, Ordering::SeqCst);
    let app = App::<AsyncTestCli, AsyncTestCli, clifacet::Plain, AsyncTestCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = clifacet::cli::block_on(app.run_with_async(
        ["asynccli", "--prefix", "prod", "fetch", "users"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    ));
    assert_eq!(code, 0);
    assert_eq!(
        CX_INIT_COUNT.load(Ordering::SeqCst),
        1,
        "Context initialized exactly once"
    );
    let output = String::from_utf8(out).unwrap();
    assert_eq!(output, "prod:users:100\n");

    // Test compute with RunCx
    let mut out2 = Vec::new();
    let mut err2 = Vec::new();
    let code2 = clifacet::cli::block_on(app.run_with_async(
        ["asynccli", "compute", "25"],
        Capabilities::headless(),
        prompter,
        &mut out2,
        &mut err2,
    ));
    assert_eq!(code2, 0);
    let output2 = String::from_utf8(out2).unwrap();
    assert_eq!(output2, "sum:125\n");

    // Test bad_cmd triggering failure hook
    let mut out3 = Vec::new();
    let mut err3 = Vec::new();
    let code3 = clifacet::cli::block_on(app.run_with_async(
        ["asynccli", "bad-cmd"],
        Capabilities::headless(),
        prompter,
        &mut out3,
        &mut err3,
    ));
    assert_ne!(code3, 0);
    let err_str = String::from_utf8(err3).unwrap();
    assert!(err_str.contains("deliberate async failure"));
}

#[derive(Clone, Default)]
struct FromCliCx {
    tag: String,
}

impl clifacet::cli::AsyncFromCli<FromCliApp> for FromCliCx {
    fn from_cli_async(
        cli: &FromCliApp,
    ) -> impl std::future::Future<Output = Result<Self, Failure>> {
        let tag = cli.tag.clone();
        async move { Ok(Self { tag }) }
    }
}

#[app(name = "from-cli-app", commands)]
struct FromCliApp {
    #[clifacet(long, default_value = "default-tag")]
    tag: String,

    #[clifacet(context)]
    cx: FromCliCx,
}

#[commands]
impl FromCliApp {
    pub async fn show(&self) -> Result<String, Failure> {
        Ok(format!("tag:{}", self.cx.tag))
    }
}

#[test]
fn test_async_from_cli_context_wiring() {
    let app = App::<FromCliApp, FromCliApp, clifacet::Plain, FromCliCx>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = clifacet::cli::block_on(app.run_with_async(
        ["from-cli-app", "--tag", "custom-tag", "show"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    ));
    assert_eq!(code, 0);
    let output = String::from_utf8(out).unwrap();
    assert_eq!(output, "tag:custom-tag\n");
}
