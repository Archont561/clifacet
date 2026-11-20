#![cfg(all(feature = "skill", feature = "skill-minijinja"))]
#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self,
    clippy::assert_is_empty
)]

use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::skill::{SkillProvider, WriteSkillOptions, execute_write_skill};
use clifacet::{App, Failure, app, commands};
use std::path::PathBuf;

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "clifacet-write-skill-{name}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("a writable temp directory");
        Self { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.path).ok();
    }
}

#[app(
    name = "ops-cli",
    version = "1.0.0",
    about = "Operations management CLI",
    skill,
    commands
)]
struct OpsCli {
    #[clifacet(long)]
    verbose: bool,
}

#[commands]
impl OpsCli {
    /// Ping service health check
    pub fn ping(&self) -> Result<String, Failure> {
        Ok("pong".into())
    }

    /// Deploy container image to target cluster
    pub fn deploy(
        &self,
        image: String,
        #[clifacet(long, default_value = "1")] replicas: u32,
    ) -> Result<String, Failure> {
        Ok(format!("deployed {image} with {replicas} replicas"))
    }
}

#[app(
    name = "single-cli",
    version = "0.5.0",
    about = "Single command line tool"
)]
struct SingleCli {
    #[clifacet(long)]
    dry_run: bool,
}

impl clifacet::Handler<()> for SingleCli {
    type Input = Self;
    type Output = String;

    fn handle(&self, _input: Self::Input, _cx: &()) -> Result<Self::Output, Failure> {
        Ok("executed single command".into())
    }
}

#[test]
fn test_write_skill_stdout_streaming() {
    let app = App::<OpsCli>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["ops-cli", "write-skill", "--stdout"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(
        err.is_empty(),
        "stderr should be empty, got: {}",
        String::from_utf8_lossy(&err)
    );

    let output = String::from_utf8(out).expect("valid utf-8 output");
    assert!(output.contains("# ops-cli\n\nOperations management CLI\n\n"));
    assert!(output.contains("### `ops-cli ping`\n\nPing service health check\n"));
    assert!(output.contains("### `ops-cli deploy`\n\nDeploy container image to target cluster\n"));
    assert_eq!(output, OpsCli::render_skill().expect("rendered skill"));
}

#[test]
fn test_write_skill_custom_output_file() {
    let scratch = Scratch::new("custom-file");
    let target_path = scratch.path.join("nested/agents/skills/ops-cli/SKILL.md");
    let target_str = target_path.to_str().expect("valid utf-8 path");

    let app = App::<OpsCli>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["ops-cli", "write-skill", "-o", target_str],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty());

    let stdout_msg = String::from_utf8(out).expect("valid utf-8");
    assert!(stdout_msg.contains("wrote skill document to"));
    assert!(stdout_msg.contains(target_str));

    // Verify on-disk file
    assert!(target_path.exists());
    let file_content = std::fs::read_to_string(&target_path).expect("read written skill");
    assert_eq!(
        file_content,
        OpsCli::render_skill().expect("rendered skill")
    );
}

#[test]
fn test_write_skill_check_up_to_date() {
    let scratch = Scratch::new("check-up-to-date");
    let skill_path = scratch.path.join("SKILL.md");
    let rendered = OpsCli::render_skill().expect("rendered skill");
    std::fs::write(&skill_path, rendered).expect("write initial skill");

    let app = App::<OpsCli>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        [
            "ops-cli",
            "write-skill",
            "--check",
            "-o",
            skill_path.to_str().unwrap(),
        ],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(err.is_empty());
    let stdout_msg = String::from_utf8(out).expect("valid utf-8");
    assert!(stdout_msg.contains("is up to date"));
}

#[test]
fn test_write_skill_check_detects_drift() {
    let scratch = Scratch::new("check-drift");
    let skill_path = scratch.path.join("SKILL.md");
    std::fs::write(&skill_path, "# Outdated skill documentation\n").expect("write outdated skill");

    let app = App::<OpsCli>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        [
            "ops-cli",
            "write-skill",
            "--check",
            "-o",
            skill_path.to_str().unwrap(),
        ],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_ne!(code, 0, "drift check should exit non-zero");
    let stderr_msg = String::from_utf8(err).expect("valid utf-8");
    assert!(
        stderr_msg.contains("has drifted from current CLI definition"),
        "stderr should explain drift: {stderr_msg}"
    );
}

#[test]
fn test_write_skill_check_missing_file_reports_drift() {
    let scratch = Scratch::new("missing-file");
    let non_existent = scratch.path.join("non_existent_skill.md");

    let app = App::<OpsCli>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        [
            "ops-cli",
            "write-skill",
            "--check",
            "-o",
            non_existent.to_str().unwrap(),
        ],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_ne!(code, 0, "missing file check should exit non-zero");
    let stderr_msg = String::from_utf8(err).expect("valid utf-8");
    assert!(
        stderr_msg.contains("does not exist (drift detected)"),
        "stderr should explain missing file: {stderr_msg}"
    );
}

#[test]
fn test_write_skill_inherent_and_trait_methods() {
    let rendered = OpsCli::render_skill().expect("rendered skill");

    // Test stdout option
    let opts_stdout = WriteSkillOptions {
        stdout: true,
        output: None,
        check: false,
    };
    let stdout_result = OpsCli::write_skill(&opts_stdout).expect("write skill stdout");
    assert_eq!(stdout_result, rendered);

    let trait_stdout =
        <OpsCli as SkillProvider>::write_skill(&opts_stdout).expect("trait write skill stdout");
    assert_eq!(trait_stdout, rendered);

    // Test custom file writing and check
    let scratch = Scratch::new("inherent-methods");
    let skill_file = scratch.path.join("inherent/SKILL.md");

    let opts_write = WriteSkillOptions {
        stdout: false,
        output: Some(skill_file.clone()),
        check: false,
    };
    let write_msg = OpsCli::write_skill(&opts_write).expect("write skill to file");
    assert!(write_msg.contains("wrote skill document to"));
    assert!(skill_file.exists());

    // Check matching
    let opts_check = WriteSkillOptions {
        stdout: false,
        output: Some(skill_file.clone()),
        check: true,
    };
    let check_msg = OpsCli::write_skill(&opts_check).expect("check match");
    assert!(check_msg.contains("is up to date"));

    // Check drift
    std::fs::write(&skill_file, "drift").expect("overwrite");
    let drift_err = OpsCli::write_skill(&opts_check).expect_err("should drift");
    assert!(drift_err.message().contains("has drifted"));
}

#[test]
fn test_write_skill_app_without_commands_feature() {
    let app = App::<SingleCli>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["single-cli", "write-skill", "--stdout"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    assert!(
        err.is_empty(),
        "stderr should be empty, got: {}",
        String::from_utf8_lossy(&err)
    );
    let output = String::from_utf8(out).expect("valid utf-8");
    assert!(output.contains("# single-cli\n\nSingle command line tool\n\n"));
}

#[test]
fn test_write_skill_help_subcommand() {
    let app = App::<OpsCli>::for_cli();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app.run_with(
        ["ops-cli", "write-skill", "--help"],
        Capabilities::headless(),
        prompter,
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0);
    let output = String::from_utf8(out).expect("valid utf-8");
    assert!(output.contains("--stdout"), "help should document --stdout");
    assert!(output.contains("--output"), "help should document --output");
    assert!(output.contains("--check"), "help should document --check");
}

#[test]
fn test_execute_write_skill_default_path_formatting() {
    let opts = WriteSkillOptions::default();
    let rendered = "# tool\n";

    // When output is None, path defaults to .agents/skills/{cli_name}/SKILL.md
    // We test with check: true on non-existent path
    let err = execute_write_skill(
        "my-tool",
        rendered,
        &WriteSkillOptions {
            check: true,
            ..opts
        },
    )
    .expect_err("should detect missing default path");

    assert!(err.message().contains(".agents/skills/my-tool/SKILL.md"));
    assert!(err.message().contains("does not exist (drift detected)"));
}
