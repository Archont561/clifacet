use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Cli, Failure, Globals, Handler};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TestInput {
    name: Option<String>,
    fail: bool,
}

#[clifacet::command(input = TestInput)]
#[derive(Debug, Clone)]
#[command(name = "test-app", version = "1.0.0", about = "Test application")]
struct TestCli {
    #[clifacet(long)]
    name: Option<String>,
    #[clifacet(long)]
    fail: bool,
    #[command(flatten)]
    #[clifacet(skip)]
    globals: Globals,
}

#[derive(Clone)]
struct TestCx {
    log: Rc<RefCell<Vec<String>>>,
}

impl Cli for TestCli {
    type Context = TestCx;

    fn init_context(&self) -> Result<Self::Context, Failure> {
        if self.name.as_deref() == Some("ctx-fail") {
            return Err(Failure::execution("failed to init context"));
        }
        Ok(TestCx {
            log: Rc::new(RefCell::new(Vec::new())),
        })
    }

    fn before_run(&self, cmd: &str, cx: &Self::Context) -> Result<(), Failure> {
        cx.log.borrow_mut().push(format!("cli::before_run:{cmd}"));
        Ok(())
    }

    fn after_success(&self, cmd: &str, cx: &Self::Context) -> Result<(), Failure> {
        cx.log
            .borrow_mut()
            .push(format!("cli::after_success:{cmd}"));
        Ok(())
    }

    fn after_failure(&self, cmd: &str, err: &Failure, cx: Option<&Self::Context>) {
        if let Some(cx) = cx {
            cx.log
                .borrow_mut()
                .push(format!("cli::after_failure:{cmd}:{}", err.message()));
        }
    }
}

impl Handler<TestCx> for TestCli {
    type Input = TestInput;
    type Output = String;

    fn handle(&self, _input: Self::Input, cx: &TestCx) -> Result<Self::Output, Failure> {
        cx.log.borrow_mut().push("handle".into());
        if self.fail {
            return Err(Failure::execution("command failed"));
        }
        Ok("success".into())
    }
}

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[test]
fn test_lifecycle_hooks_success() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let events_clone = Rc::clone(&events);
    let events_clone2 = Rc::clone(&events);
    let events_clone3 = Rc::clone(&events);

    let app = App::<TestCli, TestCli, clifacet::Plain, TestCx>::for_cli()
        .before_run(move |cmd, cx| {
            events_clone.borrow_mut().push(format!("before:{cmd}"));
            cx.log.borrow_mut().push("before".into());
            Ok(())
        })
        .after_success(move |cmd, cx| {
            events_clone2
                .borrow_mut()
                .push(format!("after_success:{cmd}"));
            cx.log.borrow_mut().push("after_success".into());
            Ok(())
        })
        .after_failure(move |cmd, err, _cx| {
            events_clone3
                .borrow_mut()
                .push(format!("after_failure:{cmd}:{err}"));
        });

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-app", "--name", "alice"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert_eq!(
        *events.borrow(),
        vec!["before:test-app", "after_success:test-app"]
    );
}

#[test]
fn test_lifecycle_hooks_command_failure() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let events_clone = Rc::clone(&events);
    let events_clone2 = Rc::clone(&events);
    let events_clone3 = Rc::clone(&events);

    let app = App::<TestCli, TestCli, clifacet::Plain, TestCx>::for_cli()
        .before_run(move |cmd, cx| {
            events_clone.borrow_mut().push(format!("before:{cmd}"));
            cx.log.borrow_mut().push("before".into());
            Ok(())
        })
        .after_success(move |cmd, _cx| {
            events_clone2
                .borrow_mut()
                .push(format!("after_success:{cmd}"));
            Ok(())
        })
        .after_failure(move |cmd, err, cx| {
            let has_cx = cx.is_some();
            events_clone3
                .borrow_mut()
                .push(format!("after_failure:{cmd}:{}:{has_cx}", err.message()));
        });

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-app", "--fail"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_ne!(code, 0);
    assert_eq!(
        *events.borrow(),
        vec![
            "before:test-app",
            "after_failure:test-app:command failed:true"
        ]
    );
}

#[test]
fn test_lifecycle_hooks_before_run_failure() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let events_clone = Rc::clone(&events);

    let app = App::<TestCli, TestCli, clifacet::Plain, TestCx>::for_cli()
        .before_run(|_cmd, _cx| Err(Failure::execution("guard failed")))
        .after_failure(move |cmd, err, cx| {
            events_clone.borrow_mut().push(format!(
                "after_failure:{cmd}:{}:{has_cx}",
                err.message(),
                has_cx = cx.is_some()
            ));
        });

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-app"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_ne!(code, 0);
    assert_eq!(
        *events.borrow(),
        vec!["after_failure:test-app:guard failed:true"]
    );
}

#[test]
fn test_lifecycle_context_init_failure() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let events_clone = Rc::clone(&events);

    let app = App::<TestCli, TestCli, clifacet::Plain, TestCx>::for_cli().after_failure(
        move |cmd, err, cx| {
            events_clone.borrow_mut().push(format!(
                "after_failure:{cmd}:{}:{has_cx}",
                err.message(),
                has_cx = cx.is_some()
            ));
        },
    );

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-app", "--name", "ctx-fail"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_ne!(code, 0);
    assert_eq!(
        *events.borrow(),
        vec!["after_failure:test-app:failed to init context:false"]
    );
}

#[test]
fn test_trait_and_builder_hooks_order() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let events_clone = Rc::clone(&events);
    let events_clone2 = Rc::clone(&events);
    let captured_log = Rc::new(RefCell::new(None));
    let captured_log_clone = Rc::clone(&captured_log);

    let app = App::<TestCli, TestCli, clifacet::Plain, TestCx>::for_cli()
        .before_run(move |cmd, cx| {
            events_clone
                .borrow_mut()
                .push(format!("builder_before:{cmd}"));
            cx.log.borrow_mut().push("builder_before".into());
            *captured_log_clone.borrow_mut() = Some(Rc::clone(&cx.log));
            Ok(())
        })
        .after_success(move |cmd, cx| {
            events_clone2
                .borrow_mut()
                .push(format!("builder_after:{cmd}"));
            cx.log.borrow_mut().push("builder_after".into());
            Ok(())
        });

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-app"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert_eq!(
        *events.borrow(),
        vec!["builder_before:test-app", "builder_after:test-app"]
    );
    let log = captured_log.borrow().as_ref().unwrap().borrow().clone();
    assert_eq!(
        log,
        vec![
            "cli::before_run:test-app",
            "builder_before",
            "handle",
            "cli::after_success:test-app",
            "builder_after"
        ]
    );
}

#[test]
fn test_from_cli() {
    use clifacet::FromCli;

    struct ExtractedConfig {
        name: String,
    }

    impl FromCli<TestCli> for ExtractedConfig {
        fn from_cli(cli: &TestCli) -> Result<Self, Failure> {
            Ok(Self {
                name: cli.name.clone().unwrap_or_else(|| "default".into()),
            })
        }
    }

    let fn_ptr: fn() -> std::process::ExitCode = clifacet::run_cli::<TestCli>;
    let _ = fn_ptr;

    let cli = TestCli {
        name: Some("custom".into()),
        fail: false,
        globals: Globals::default(),
    };

    let extracted = ExtractedConfig::from_cli(&cli).expect("extracted");
    assert_eq!(extracted.name, "custom");
}

#[test]
fn test_lazy_context_skipped_on_help_and_version() {
    // When --help is passed, init_context should never be called.
    // If init_context was called with ctx-fail it would fail, but with --help it should output help and exit 0.
    let app = App::<TestCli, TestCli, clifacet::Plain, TestCx>::for_cli();

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-app", "--name", "ctx-fail", "--help"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    let output = String::from_utf8_lossy(&out);
    assert!(output.contains("--help"), "Help output expected: {output}");
}
