use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Failure, FromCli, Handler, app};
use std::cell::RefCell;
use std::rc::Rc;

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Default, Clone)]
struct DefaultContext {
    tag: String,
}

#[app(
    name = "test-default-ctx",
    version = "1.0.0",
    about = "Test with Default context"
)]
struct AppWithDefault {
    #[clifacet(long)]
    name: Option<String>,

    #[clifacet(context)]
    cx: DefaultContext,
}

impl Handler<DefaultContext> for AppWithDefault {
    type Input = Self;
    type Output = String;

    fn handle(&self, _input: Self::Input, _cx: &DefaultContext) -> Result<Self::Output, Failure> {
        // Access self.cx directly via Deref!
        let tag = if self.cx.tag.is_empty() {
            "default-tag"
        } else {
            &self.cx.tag
        };
        Ok(format!("hello from {tag}"))
    }
}

#[test]
fn test_app_macro_with_default_context() {
    let app = App::<AppWithDefault, AppWithDefault, clifacet::Plain, DefaultContext>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-default-ctx", "--name", "world"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap(), "hello from default-tag\n");
}

#[derive(Clone)]
struct FromCliContext {
    configured_user: String,
}

#[app(name = "test-fromcli-ctx", version)]
struct AppWithFromCli {
    #[clifacet(long)]
    user: String,

    #[clifacet(context)]
    cx: FromCliContext,
}

impl FromCli<AppWithFromCli> for FromCliContext {
    fn from_cli(cli: &AppWithFromCli) -> Result<Self, Failure> {
        Ok(Self {
            configured_user: cli.user.clone(),
        })
    }
}

impl Handler<FromCliContext> for AppWithFromCli {
    type Input = Self;
    type Output = String;

    fn handle(&self, _input: Self::Input, _cx: &FromCliContext) -> Result<Self::Output, Failure> {
        Ok(format!("user is {}", self.cx.configured_user))
    }
}

#[test]
fn test_app_macro_with_from_cli_context() {
    let app = App::<AppWithFromCli, AppWithFromCli, clifacet::Plain, FromCliContext>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-fromcli-ctx", "--user", "bob"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap(), "user is bob\n");
}

#[derive(Clone)]
struct CustomContext {
    val: i32,
}

#[allow(clippy::unnecessary_wraps)]
fn build_custom_ctx(cli: &AppWithCustom) -> Result<CustomContext, Failure> {
    Ok(CustomContext {
        val: cli.multiplier * 10,
    })
}

#[app(name = "test-custom-ctx", version = "2.0.0")]
struct AppWithCustom {
    #[clifacet(long, default_value = "5")]
    multiplier: i32,

    #[clifacet(context = build_custom_ctx)]
    cx: CustomContext,
}

impl Handler<CustomContext> for AppWithCustom {
    type Input = Self;
    type Output = String;

    fn handle(&self, _input: Self::Input, _cx: &CustomContext) -> Result<Self::Output, Failure> {
        Ok(format!("val is {}", self.cx.val))
    }
}

#[test]
fn test_app_macro_with_custom_context_factory() {
    let app = App::<AppWithCustom, AppWithCustom, clifacet::Plain, CustomContext>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-custom-ctx", "--multiplier", "3"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap(), "val is 30\n");
}

#[app]
struct AppWithoutContext {
    #[clifacet(long)]
    shout: bool,
}

impl Handler<()> for AppWithoutContext {
    type Input = Self;
    type Output = String;

    fn handle(&self, _input: Self::Input, _cx: &()) -> Result<Self::Output, Failure> {
        Ok(if self.shout {
            "LOUD".into()
        } else {
            "quiet".into()
        })
    }
}

#[test]
fn test_app_macro_without_context() {
    let app = App::<AppWithoutContext, AppWithoutContext, clifacet::Plain, ()>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["app", "--shout"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap(), "LOUD\n");
}

#[derive(clap::Subcommand, Clone)]
enum SubCmd {
    Ping,
}

impl clifacet::facet::__derive::Tree for SubCmd {
    type Input = Self;
    fn into_input(self) -> Result<Self::Input, Failure> {
        Ok(self)
    }
}

impl clifacet::prompt::Collect for SubCmd {
    fn collect(self, _session: &mut clifacet::prompt::Session) -> Result<Self, Failure> {
        Ok(self)
    }
}

#[app(name = "test-subcmd")]
struct AppWithSub {
    #[clifacet(subcommand)]
    cmd: Option<SubCmd>,
}

impl Handler<()> for AppWithSub {
    type Input = Self;
    type Output = String;

    fn handle(&self, _input: Self::Input, _cx: &()) -> Result<Self::Output, Failure> {
        match &self.cmd {
            Some(SubCmd::Ping) => Ok("pong".into()),
            None => Ok("none".into()),
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn hook_before(_cli: &AppWithHooks, _cmd: &str, cx: &HookContext) -> Result<(), Failure> {
    cx.events.borrow_mut().push("before".into());
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn hook_success(_cli: &AppWithHooks, _cmd: &str, cx: &HookContext) -> Result<(), Failure> {
    cx.events.borrow_mut().push("success".into());
    Ok(())
}

fn hook_failure(_cli: &AppWithHooks, _cmd: &str, _err: &Failure, cx: Option<&HookContext>) {
    if let Some(cx) = cx {
        cx.events.borrow_mut().push("failure".into());
    }
}

#[derive(Clone, Default)]
struct HookContext {
    events: Rc<RefCell<Vec<String>>>,
}

#[app(
    name = "test-hooks",
    before_run = hook_before,
    after_success = hook_success,
    after_failure = hook_failure
)]
struct AppWithHooks {
    #[clifacet(long)]
    fail: bool,

    #[clifacet(context)]
    cx: HookContext,
}

impl Handler<HookContext> for AppWithHooks {
    type Input = Self;
    type Output = String;

    fn handle(&self, _input: Self::Input, _cx: &HookContext) -> Result<Self::Output, Failure> {
        if self.fail {
            return Err(Failure::execution("deliberate failure"));
        }
        Ok("ok".into())
    }
}

#[test]
fn test_app_macro_with_subcommand() {
    let app = App::<AppWithSub, AppWithSub, clifacet::Plain, ()>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-subcmd", "ping"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap(), "pong\n");
}

#[test]
fn test_app_macro_with_lifecycle_hooks() {
    let app = App::<AppWithHooks, AppWithHooks, clifacet::Plain, HookContext>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        ["test-hooks"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);

    let mut out_fail = Vec::new();
    let mut err_fail = Vec::new();
    let code_fail = app.run_with(
        ["test-hooks", "--fail"],
        Capabilities::headless(),
        prompter,
        &mut out_fail,
        &mut err_fail,
    );
    assert_ne!(code_fail, 0);
}
