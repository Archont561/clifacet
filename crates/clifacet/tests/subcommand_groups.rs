#![allow(
    clippy::pedantic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value
)]

use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Failure, app, commands, group};
use std::sync::{Arc, Mutex};

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Debug, Clone)]
pub struct AppDatabase {
    log: Arc<Mutex<Vec<String>>>,
    table: String,
}

impl From<&AppDatabase> for AppDatabase {
    fn from(db: &AppDatabase) -> Self {
        db.clone()
    }
}

#[derive(Clone)]
pub struct AppContext {
    db: AppDatabase,
    server_port: u16,
}

impl AsRef<u16> for AppContext {
    fn as_ref(&self) -> &u16 {
        &self.server_port
    }
}

impl From<&AppContext> for AppDatabase {
    fn from(cx: &AppContext) -> Self {
        cx.db.clone()
    }
}

// Child group: Roles
#[group(commands)]
pub struct RolesGroup {
    #[clifacet(extract)]
    db: AppDatabase,
}

#[commands(group)]
impl RolesGroup {
    /// Add role to system.
    pub fn assign(&self, user: String, role: String) -> Result<String, Failure> {
        self.db
            .log
            .lock()
            .unwrap()
            .push(format!("role:assign:{user}:{role}:table={}", self.db.table));
        Ok(format!("assigned {user} to {role}"))
    }
}

// Child group: Users
#[group(commands)]
pub struct UsersGroup {
    #[clifacet(extract)]
    db: AppDatabase,
}

#[commands(group)]
impl UsersGroup {
    /// List all users.
    pub fn list(&self) -> Result<String, Failure> {
        self.db
            .log
            .lock()
            .unwrap()
            .push(format!("users:list:table={}", self.db.table));
        Ok("alice, bob".into())
    }

    /// Subcommand group forwarder for roles.
    pub fn roles(&self, #[clifacet(subcommand)] roles: RolesGroup) -> Result<String, Failure> {
        roles.init_from_parent(&*self.db);
        roles.execute()
    }
}

#[app(name = "nestcli", version = "1.0.0", commands)]
pub struct NestCli {
    #[clifacet(long)]
    dry_mode: bool,

    #[clifacet(context = build_context)]
    cx: AppContext,
}

fn build_context(_cli: &NestCli) -> Result<AppContext, Failure> {
    Ok(AppContext {
        db: AppDatabase {
            log: Arc::new(Mutex::new(Vec::new())),
            table: "main_tbl".into(),
        },
        server_port: 8080,
    })
}

#[commands]
impl NestCli {
    /// Top-level status.
    pub fn status(&self) -> Result<String, Failure> {
        Ok(format!("ok (dry={})", self.dry_mode))
    }

    /// Mount users subcommand group.
    pub fn users(&self, #[clifacet(subcommand)] users: UsersGroup) -> Result<String, Failure> {
        users.init_from_parent(&*self.cx);
        users.execute()
    }
}

#[test]
fn test_nested_subcommand_dispatch_and_context_slicing() {
    let app = App::<NestCli, NestCli, clifacet::Plain, AppContext>::for_cli();

    // 1. Level 1 command: `status`
    let mut out1 = Vec::new();
    let mut err1 = Vec::new();
    let code1 = app.run_with(
        ["nestcli", "--dry-mode", "status"],
        Capabilities::headless(),
        prompter,
        &mut out1,
        &mut err1,
    );
    assert_eq!(code1, 0);
    assert_eq!(String::from_utf8(out1).unwrap(), "ok (dry=true)\n");

    // 2. Level 2 command: `users list`
    let mut out2 = Vec::new();
    let mut err2 = Vec::new();
    let code2 = app.run_with(
        ["nestcli", "users", "list"],
        Capabilities::headless(),
        prompter,
        &mut out2,
        &mut err2,
    );
    assert_eq!(code2, 0);
    assert_eq!(String::from_utf8(out2).unwrap(), "alice, bob\n");

    // 3. Level 3 nested command: `users roles assign alice admin`
    let mut out3 = Vec::new();
    let mut err3 = Vec::new();
    let code3 = app.run_with(
        ["nestcli", "users", "roles", "assign", "alice", "admin"],
        Capabilities::headless(),
        prompter,
        &mut out3,
        &mut err3,
    );
    assert_eq!(code3, 0);
    assert_eq!(
        String::from_utf8(out3).unwrap(),
        "assigned alice to admin\n"
    );

    // 4. Test nested help generation at various levels
    let mut help_out = Vec::new();
    let mut help_err = Vec::new();
    let help_code = app.run_with(
        ["nestcli", "users", "roles", "--help"],
        Capabilities::headless(),
        prompter,
        &mut help_out,
        &mut help_err,
    );
    assert_eq!(help_code, 0);
    // 5. Test leaf command help with arguments
    let mut cmd_help_out = Vec::new();
    let mut cmd_help_err = Vec::new();
    let cmd_help_code = app.run_with(
        ["nestcli", "users", "roles", "assign", "--help"],
        Capabilities::headless(),
        prompter,
        &mut cmd_help_out,
        &mut cmd_help_err,
    );
    assert_eq!(cmd_help_code, 0);
    let cmd_help_str = String::from_utf8(cmd_help_out).unwrap();
    assert!(
        cmd_help_str.contains("<USER>"),
        "Leaf help should list <USER> arg:\n{cmd_help_str}"
    );
    assert!(
        cmd_help_str.contains("<ROLE>"),
        "Leaf help should list <ROLE> arg:\n{cmd_help_str}"
    );
}
