#![cfg(feature = "tokio")]
#![allow(
    clippy::pedantic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use clifacet::r#async::{Cancel, Executor, StdExecutor, TokioExecutor};
use clifacet::policy::Capabilities;
use clifacet::prompt::Scripted;
use clifacet::{App, Failure, app, commands};

#[derive(Clone, Default)]
struct AsyncContext {
    ready: Arc<AtomicBool>,
}

#[app(name = "testapp", about = "Async test app", commands)]
struct AsyncApp {
    #[clifacet(context = build_context)]
    cx: AsyncContext,
}

fn build_context(_cli: &AsyncApp) -> Result<AsyncContext, Failure> {
    Ok(AsyncContext {
        ready: Arc::new(AtomicBool::new(true)),
    })
}

#[commands]
impl AsyncApp {
    /// Hello command.
    pub fn hello(&self) -> Result<String, Failure> {
        let is_ready = self.cx.ready.load(Ordering::SeqCst);
        Ok(format!("hello (ready={is_ready})"))
    }

    /// Long running operation simulated.
    pub async fn fetch(&self, resource: String) -> Result<String, Failure> {
        Ok(format!("fetched:{resource}"))
    }
}

#[test]
fn test_dyn_executor_and_default_tokio_executor() {
    let arc_exec = Arc::new(StdExecutor);
    let cancel = Cancel::new();
    let res = arc_exec.run(
        Box::pin(async { Ok::<_, Failure>("dyn".to_string()) }),
        cancel.clone(),
    );
    assert_eq!(res.unwrap(), "dyn");

    let def = TokioExecutor::default();
    let res_def = def.run(Box::pin(async { Ok::<_, Failure>(123) }), cancel.clone());
    assert_eq!(res_def.unwrap(), 123);

    let multi = TokioExecutor::multi_thread();
    let res_multi = multi.run(Box::pin(async { Ok::<_, Failure>(456) }), cancel);
    assert_eq!(res_multi.unwrap(), 456);
}

#[test]
fn test_app_builder_executor() {
    let app =
        App::<AsyncApp, AsyncApp, clifacet::Plain, AsyncContext>::for_cli().executor(StdExecutor);
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let code = app
            .run_with_async(
                ["testapp", "hello"],
                Capabilities::headless(),
                || Box::new(Scripted::default()),
                &mut out,
                &mut err,
            )
            .await;

        assert_eq!(code, 0);
        assert_eq!(String::from_utf8(out).unwrap(), "hello (ready=true)\n");
    });
}

#[test]
fn test_tokio_executor_current_thread_and_multi_thread() {
    let current = TokioExecutor::current_thread();
    let cancel = Cancel::new();
    let res = current.run(Box::pin(async { Ok::<_, Failure>(42) }), cancel.clone());
    assert_eq!(res.unwrap(), 42);

    let multi = TokioExecutor::multi_thread();
    let res_multi = multi.run(Box::pin(async { Ok::<_, Failure>(99) }), cancel);
    assert_eq!(res_multi.unwrap(), 99);
}

#[test]
fn test_tokio_executor_from_handle() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let executor = TokioExecutor::from_handle(rt.handle().clone());
    let cancel = Cancel::new();

    let res = executor.run(
        Box::pin(async { Ok::<_, Failure>("from_handle".to_string()) }),
        cancel,
    );
    assert_eq!(res.unwrap(), "from_handle");
}

#[test]
fn test_app_run_with_async_executes_in_runtime() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    rt.block_on(async {
        let app = App::<AsyncApp, AsyncApp, clifacet::Plain, AsyncContext>::for_cli();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let code = app
            .run_with_async(
                ["testapp", "hello"],
                Capabilities::headless(),
                || Box::new(Scripted::default()),
                &mut out,
                &mut err,
            )
            .await;

        assert_eq!(code, 0);
        assert_eq!(String::from_utf8(out).unwrap(), "hello (ready=true)\n");
    });
}

#[test]
fn test_sync_app_run_inside_runtime_returns_failure_diagnostic() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    rt.block_on(async {
        let app = App::<AsyncApp, AsyncApp, clifacet::Plain, AsyncContext>::for_cli();
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Calling synchronous run_with inside an active Tokio runtime must return Failure::execution
        // with an actionable diagnostic rather than panicking.
        let code = app.run_with(
            ["testapp", "hello"],
            Capabilities::headless(),
            || Box::new(Scripted::default()),
            &mut out,
            &mut err,
        );

        assert_ne!(code, 0);
        let err_str = String::from_utf8(err).unwrap();
        assert!(
            err_str.contains("invoking synchronous App::run() inside an active Tokio runtime"),
            "Unexpected error message: {err_str}"
        );
    });
}

#[test]
fn test_tokio_executor_inside_active_runtime_returns_failure() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    rt.block_on(async {
        let executor = TokioExecutor::current_thread();
        let cancel = Cancel::new();
        let res = executor.run(Box::pin(async { Ok::<_, Failure>(1) }), cancel);

        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(
            err.message().contains("inside an active Tokio runtime"),
            "Unexpected error message: {}",
            err.message()
        );
    });
}
