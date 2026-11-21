#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self,
    clippy::unused_async
)]

use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Failure, app, commands};
use std::cell::RefCell;
use std::rc::Rc;

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Clone)]
struct TestContext {
    log: Rc<RefCell<Vec<String>>>,
    multiplier: i32,
}

#[app(name = "mycli", version = "1.0.0", about = "Controller app", commands)]
struct MyCli {
    #[clifacet(long)]
    verbose: bool,

    #[clifacet(context = build_cx)]
    cx: TestContext,
}

fn build_cx(_cli: &MyCli) -> Result<TestContext, Failure> {
    Ok(TestContext {
        log: Rc::new(RefCell::new(Vec::new())),
        multiplier: 3,
    })
}

#[commands]
impl MyCli {
    /// Deploy a service to environment.
    pub fn deploy(
        &self,
        service: String,
        #[clifacet(long, default_value = "1")] replicas: u32,
    ) -> Result<(), Failure> {
        self.cx.log.borrow_mut().push(format!(
            "deploy:{service}:{replicas}:verbose={}",
            self.verbose
        ));
        Ok(())
    }

    /// Compute score with multiplier.
    pub fn compute(&self, base: i32) -> Result<i32, Failure> {
        let res = base * self.cx.multiplier;
        self.cx.log.borrow_mut().push(format!("compute:{res}"));
        Ok(res)
    }

    /// Say hello message.
    pub fn hello(&self) -> String {
        format!("hello world (verbose={})", self.verbose)
    }

    /// Method returning unit directly.
    pub fn ping(&self) {
        self.cx.log.borrow_mut().push("ping".into());
    }

    /// Custom error return.
    pub fn fail(&self) -> Result<(), String> {
        Err("deliberate failure".into())
    }

    /// Asynchronous fetch test.
    pub async fn async_fetch(&self, resource: String) -> Result<String, Failure> {
        Ok(format!("fetched async:{resource}"))
    }
}

#[test]
fn test_commands_dispatch_and_self_context() {
    let app = App::<MyCli, MyCli, clifacet::Plain, TestContext>::for_cli();

    // 1. Test deploy command with positional arg and flag
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        [
            "mycli",
            "--verbose",
            "deploy",
            "api-server",
            "--replicas",
            "4",
        ],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);

    // 2. Test compute command returning i32
    let mut out2 = Vec::new();
    let mut err2 = Vec::new();
    let code2 = app.run_with(
        ["mycli", "compute", "7"],
        Capabilities::headless(),
        prompter,
        &mut out2,
        &mut err2,
    );
    assert_eq!(code2, 0);
    assert_eq!(String::from_utf8(out2).unwrap(), "21\n");

    // 3. Test hello command returning String
    let mut out3 = Vec::new();
    let mut err3 = Vec::new();
    let code3 = app.run_with(
        ["mycli", "--verbose", "hello"],
        Capabilities::headless(),
        prompter,
        &mut out3,
        &mut err3,
    );
    assert_eq!(code3, 0);
    assert_eq!(
        String::from_utf8(out3).unwrap(),
        "hello world (verbose=true)\n"
    );

    // 4. Test ping command returning ()
    let mut out4 = Vec::new();
    let mut err4 = Vec::new();
    let code4 = app.run_with(
        ["mycli", "ping"],
        Capabilities::headless(),
        prompter,
        &mut out4,
        &mut err4,
    );
    assert_eq!(code4, 0);

    // 5. Test fail command returning Result<(), String>
    let mut out5 = Vec::new();
    let mut err5 = Vec::new();
    let code5 = app.run_with(
        ["mycli", "fail"],
        Capabilities::headless(),
        prompter,
        &mut out5,
        &mut err5,
    );
    assert_ne!(code5, 0);
    assert!(
        String::from_utf8(err5)
            .unwrap()
            .contains("deliberate failure")
    );

    // 6. Test async command returning String
    let mut out6 = Vec::new();
    let mut err6 = Vec::new();
    let code6 = app.run_with(
        ["mycli", "async-fetch", "users"],
        Capabilities::headless(),
        prompter,
        &mut out6,
        &mut err6,
    );
    assert_eq!(code6, 0);
    assert_eq!(String::from_utf8(out6).unwrap(), "fetched async:users\n");
}

#[test]
fn test_commands_help_text_generation() {
    let app = App::<MyCli, MyCli, clifacet::Plain, TestContext>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["mycli", "--help"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let output = String::from_utf8(out).unwrap();
    assert!(
        output.contains("deploy"),
        "Expected deploy in help:\n{output}"
    );
    assert!(
        output.contains("Deploy a service to environment"),
        "Expected docstring in help:\n{output}"
    );
    assert!(
        output.contains("compute"),
        "Expected compute in help:\n{output}"
    );
    assert!(
        output.contains("Compute score with multiplier"),
        "Expected compute docstring in help:\n{output}"
    );
}
