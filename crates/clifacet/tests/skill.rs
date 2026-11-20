//! Integration tests for pluggable skill rendering and feature boundary (CLI-74, decision-25).

#![cfg(feature = "skill")]
#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use clifacet::schema::CliMetadata;
use clifacet::skill::{SkillProvider, SkillRenderer};
use clifacet::{Failure, app, commands};
use std::sync::Arc;

/// A custom Markdown skill renderer implemented by a user.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct MarkdownSkillRenderer {
    pub header_level: usize,
}

impl SkillRenderer for MarkdownSkillRenderer {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure> {
        let mut out = String::new();
        let hashes = "#".repeat(self.header_level.max(1));
        out.push_str(&format!("{hashes} {}\n\n", cli.name));

        if let Some(about) = &cli.about {
            out.push_str(&format!("{about}\n\n"));
        }

        if !cli.arguments.is_empty() {
            out.push_str("### Options\n\n");
            for arg in &cli.arguments {
                let flag_name = arg.long.as_deref().unwrap_or(&arg.id);
                let help_text = arg.help.as_deref().unwrap_or("No description");
                out.push_str(&format!("- `--{flag_name}`: {help_text}\n"));
            }
            out.push('\n');
        }

        if !cli.subcommands.is_empty() {
            out.push_str("### Subcommands\n\n");
            for sub in &cli.subcommands {
                let sub_about = sub.about.as_deref().unwrap_or("No description");
                out.push_str(&format!("- `{}`: {sub_about}\n", sub.name));
            }
        }

        Ok(out)
    }
}

/// A custom JSON-based skill renderer.
#[derive(Default, Debug, Clone)]
struct JsonSkillRenderer;

impl SkillRenderer for JsonSkillRenderer {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure> {
        serde_json::to_string_pretty(cli)
            .map_err(|e| Failure::execution(format!("JSON rendering failed: {e}")))
    }
}

#[app(name = "fleet", about = "Fleet management CLI tool", version = "2.1.0", skill = MarkdownSkillRenderer, commands)]
struct FleetApp {
    /// Verbose logs
    #[clifacet(short, long)]
    pub verbose: bool,

    /// Target cluster name
    #[clifacet(long, default_value = "main-cluster")]
    pub cluster: String,
}

#[commands]
impl FleetApp {
    /// List deployed containers
    pub fn ps(&self) -> &'static str {
        "container1, container2"
    }

    /// Scale service instances
    pub fn scale(&self, service: String, count: u32) -> Result<String, Failure> {
        Ok(format!("scaled {service} to {count}"))
    }
}

#[test]
fn test_custom_skill_renderer_output() {
    let renderer = MarkdownSkillRenderer { header_level: 1 };
    let meta = FleetApp::cli_metadata();

    let output = renderer.render(&meta).expect("rendered markdown");

    assert!(output.starts_with("# fleet\n\n"));
    assert!(output.contains("Fleet management CLI tool\n\n"));
    assert!(output.contains("### Options\n\n"));
    assert!(output.contains("- `--verbose`: Verbose logs\n"));
    assert!(output.contains("- `--cluster`: Target cluster name\n"));
    assert!(output.contains("### Subcommands\n\n"));
    assert!(output.contains("- `ps`: List deployed containers\n"));
    assert!(output.contains("- `scale`: Scale service instances\n"));
}

#[test]
fn test_app_macro_skill_attribute_integration() {
    // Verify FleetApp::skill_renderer() returns the configured renderer
    let renderer = FleetApp::skill_renderer();
    assert_eq!(renderer.header_level, 0);

    // Verify FleetApp::render_skill() works via default skill renderer
    let rendered = FleetApp::render_skill().expect("rendered skill");
    assert!(rendered.contains("# fleet\n\n"));
    assert!(rendered.contains("Fleet management CLI tool"));
    assert!(rendered.contains("- `ps`: List deployed containers"));

    // Verify SkillProvider trait is implemented
    let provider_rendered = <FleetApp as SkillProvider>::render_skill().expect("rendered");
    assert_eq!(rendered, provider_rendered);
}

#[test]
fn test_app_render_skill_with_alternative_renderers() {
    // Render using an alternative custom renderer (JsonSkillRenderer)
    let json_renderer = JsonSkillRenderer;
    let json_output = FleetApp::render_skill_with(&json_renderer).expect("json skill rendered");

    let parsed: serde_json::Value = serde_json::from_str(&json_output).expect("valid json");
    assert_eq!(parsed["name"], "fleet");
    assert_eq!(parsed["about"], "Fleet management CLI tool");

    // Render using a function pointer
    fn plain_renderer(cli: &CliMetadata) -> Result<String, Failure> {
        Ok(format!(
            "CLI: {} (v{})",
            cli.name,
            cli.version.as_deref().unwrap_or("?")
        ))
    }

    let plain_output = FleetApp::render_skill_with(
        &(plain_renderer as fn(&CliMetadata) -> Result<String, Failure>),
    )
    .expect("plain rendered");
    assert_eq!(plain_output, "CLI: fleet (v2.1.0)");

    // Render using Arc<dyn SkillRenderer> and Box<dyn SkillRenderer>
    let boxed: Box<dyn SkillRenderer> = Box::new(MarkdownSkillRenderer { header_level: 2 });
    let boxed_output = FleetApp::render_skill_with(&boxed).expect("boxed rendered");
    assert!(boxed_output.starts_with("## fleet\n\n"));

    let arc: Arc<dyn SkillRenderer> = Arc::new(MarkdownSkillRenderer { header_level: 3 });
    let arc_output = FleetApp::render_skill_with(&arc).expect("arc rendered");
    assert!(arc_output.starts_with("### fleet\n\n"));
}

#[app(name = "bare-app", about = "An app without skill attribute")]
struct BareApp {
    #[clifacet(long)]
    pub debug: bool,
}

#[test]
fn test_app_without_skill_attribute_supports_render_skill_with() {
    let renderer = MarkdownSkillRenderer { header_level: 1 };
    let rendered = BareApp::render_skill_with(&renderer).expect("rendered");

    assert!(rendered.starts_with("# bare-app\n\n"));
    assert!(rendered.contains("An app without skill attribute"));
    assert!(rendered.contains("- `--debug`: "));
}

// ---------------------------------------------------------------------------
// CLI-75 MiniJinja Skill Renderer and Embedded Template tests
// ---------------------------------------------------------------------------

#[cfg(feature = "skill-minijinja")]
use clifacet::group;
#[cfg(feature = "skill-minijinja")]
use clifacet::skill::{DEFAULT_SKILL_TEMPLATE, MiniJinjaSkillRenderer};

#[cfg(feature = "skill-minijinja")]
fn assert_clean_markdown(md: &str) {
    assert!(!md.is_empty(), "markdown should not be empty");
    assert!(
        md.ends_with('\n'),
        "markdown must end with exactly one newline"
    );
    assert!(
        !md.ends_with("\n\n"),
        "markdown must not end with multiple newlines"
    );

    let mut prev_empty = false;
    for (idx, line) in md.lines().enumerate() {
        assert_eq!(
            line,
            line.trim_end(),
            "line {} has trailing whitespace: {:?}",
            idx + 1,
            line
        );
        if line.is_empty() {
            assert!(
                !prev_empty,
                "line {} has multiple consecutive blank lines",
                idx + 1
            );
            prev_empty = true;
        } else {
            prev_empty = false;
        }
    }

    // Verify through rumdl and vale if binaries are available
    let rumdl_path = std::path::Path::new("/home/user/clifacet/.pixi/envs/default/bin/rumdl");
    let config_path = std::path::Path::new("/home/user/clifacet/.rumdl.toml");
    if rumdl_path.exists() && config_path.exists() {
        let temp_file = std::env::temp_dir().join(format!(
            "test_skill_{}_{}.md",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&temp_file, md).expect("write temp skill");

        let rumdl_output = std::process::Command::new(rumdl_path)
            .arg("check")
            .arg("--config")
            .arg(config_path)
            .arg(&temp_file)
            .output()
            .expect("run rumdl");

        let rumdl_stdout = String::from_utf8_lossy(&rumdl_output.stdout);
        let rumdl_stderr = String::from_utf8_lossy(&rumdl_output.stderr);
        assert!(
            rumdl_output.status.success(),
            "rumdl check failed:\nSTDOUT:\n{rumdl_stdout}\nSTDERR:\n{rumdl_stderr}"
        );

        let vale_path = std::path::Path::new("/home/user/clifacet/.pixi/envs/default/bin/vale");
        if vale_path.exists() {
            let vale_output = std::process::Command::new(vale_path)
                .arg("--ext=.md")
                .arg(&temp_file)
                .current_dir("/home/user/clifacet")
                .output()
                .expect("run vale");

            let vale_stdout = String::from_utf8_lossy(&vale_output.stdout);
            let vale_stderr = String::from_utf8_lossy(&vale_output.stderr);
            assert!(
                vale_output.status.success(),
                "vale check failed:\nSTDOUT:\n{vale_stdout}\nSTDERR:\n{vale_stderr}"
            );
        }

        let _ = std::fs::remove_file(&temp_file);
    }
}

#[cfg(feature = "skill-minijinja")]
#[test]
fn test_minijinja_embedded_default_template_and_clean_markdown() {
    assert!(
        !DEFAULT_SKILL_TEMPLATE.is_empty(),
        "embedded template must not be empty"
    );
    assert!(
        DEFAULT_SKILL_TEMPLATE.contains("name: {{ name }}"),
        "embedded template must contain frontmatter name tag"
    );
    assert!(
        DEFAULT_SKILL_TEMPLATE.contains("Non-Interactive Execution Rules"),
        "embedded template must contain non-interactive execution rules"
    );

    let renderer = MiniJinjaSkillRenderer::new();
    let meta = FleetApp::cli_metadata();
    let rendered = renderer.render(&meta).expect("rendered markdown");

    // Check frontmatter
    assert!(
        rendered.starts_with("---\nname: fleet\ndescription: Fleet management CLI tool\n---\n\n")
    );
    // Check main heading and about
    assert!(rendered.contains("# fleet\n\nFleet management CLI tool\n\n"));
    // Check non-interactive rules
    assert!(rendered.contains("## Non-Interactive Execution Rules\n\n"));
    assert!(rendered.contains("- Use non-interactive flags or explicit arguments when running unattended to avoid interactive prompts."));
    assert!(
        rendered.contains("- Pass `--json` when structured, machine-readable output is needed.")
    );
    // Check global options
    assert!(rendered.contains("## Options\n\n"));
    assert!(rendered.contains("- `--verbose, -v`: Verbose logs\n"));
    assert!(
        rendered
            .contains("- `--cluster <CLUSTER>`: Target cluster name *(default: `main-cluster`)*\n")
    );
    // Check commands
    assert!(rendered.contains("## Commands\n\n"));
    assert!(rendered.contains("### `fleet ps`\n\nList deployed containers\n"));
    assert!(rendered.contains("### `fleet scale`\n\nScale service instances\n\n#### Options\n\n"));
    assert!(rendered.contains("- `<SERVICE>`: No description. *(required)*\n"));
    assert!(rendered.contains("- `<COUNT>`: No description. *(required)*\n"));

    assert_clean_markdown(&rendered);
}

#[cfg(feature = "skill-minijinja")]
#[app(
    name = "k8s-tool",
    about = "Kubernetes cluster manager",
    skill,
    commands
)]
struct K8sTool {
    #[clifacet(long, env = "KUBECONFIG", help = "Path to kubeconfig file")]
    pub kubeconfig: Option<String>,

    #[clifacet(short, long, help = "Namespace to target", default_value = "default")]
    pub namespace: String,
}

#[cfg(feature = "skill-minijinja")]
#[commands]
impl K8sTool {
    /// Apply a Kubernetes manifest file
    pub fn apply(&self, file: String) -> Result<String, Failure> {
        Ok(format!("applied {file}"))
    }
}

#[cfg(feature = "skill-minijinja")]
#[test]
fn test_minijinja_app_with_default_skill_attribute() {
    let renderer = K8sTool::skill_renderer();
    assert_eq!(renderer, MiniJinjaSkillRenderer::new());

    let rendered = K8sTool::render_skill().expect("rendered skill");
    assert!(
        rendered
            .starts_with("---\nname: k8s-tool\ndescription: Kubernetes cluster manager\n---\n\n")
    );
    assert!(rendered.contains("# k8s-tool\n\nKubernetes cluster manager\n\n"));
    assert!(rendered.contains(
        "- `--kubeconfig <KUBECONFIG>`: Path to kubeconfig file *(environment: `KUBECONFIG`)*\n"
    ));
    assert!(
        rendered.contains(
            "- `--namespace, -n <NAMESPACE>`: Namespace to target *(default: `default`)*\n"
        )
    );
    assert!(
        rendered.contains(
            "### `k8s-tool apply`\n\nApply a Kubernetes manifest file\n\n#### Options\n\n"
        )
    );
    assert!(rendered.contains("- `<FILE>`: No description. *(required)*\n"));

    // Trait implementation matches inherent method
    let trait_rendered = <K8sTool as SkillProvider>::render_skill().expect("trait rendered");
    assert_eq!(rendered, trait_rendered);

    assert_clean_markdown(&rendered);
}

#[cfg(feature = "skill-minijinja")]
#[app(
    name = "custom-tmpl-app",
    about = "App with custom template string",
    version = "3.0.0",
    skill(
        template = "---\nname: {{ name }}\ndescription: {{ about }}\n---\n\n# Custom Doc for {{ name }}\n\n{{ about }}\n\nVersion: {{ version | default(\"0.0.0\") }}\n"
    )
)]
struct CustomTemplateApp {}

#[cfg(feature = "skill-minijinja")]
#[test]
fn test_minijinja_app_with_custom_template_attribute() {
    let renderer = CustomTemplateApp::skill_renderer();
    assert!(renderer.template().contains("# Custom Doc for {{ name }}"));

    let rendered = CustomTemplateApp::render_skill().expect("rendered skill");
    assert!(rendered.contains(
        "# Custom Doc for custom-tmpl-app\n\nApp with custom template string\n\nVersion: 3.0.0\n"
    ));

    let trait_rendered =
        <CustomTemplateApp as SkillProvider>::render_skill().expect("trait rendered");
    assert_eq!(rendered, trait_rendered);

    assert_clean_markdown(&rendered);
}

#[cfg(feature = "skill-minijinja")]
#[app(
    name = "custom-path-app",
    about = "App with custom template path",
    version = "1.2.3",
    skill(template_path = "fixtures/custom_skill.j2")
)]
struct CustomPathApp {}

#[cfg(feature = "skill-minijinja")]
#[test]
fn test_minijinja_app_with_custom_template_path_attribute() {
    let renderer = CustomPathApp::skill_renderer();
    assert!(renderer.template().contains("# Custom Skill: {{ name }}"));

    let rendered = CustomPathApp::render_skill().expect("rendered skill");
    assert!(rendered.contains("# Custom Skill: custom-path-app\n\nVersion: 1.2.3\nDescription: App with custom template path\n"));

    let trait_rendered = <CustomPathApp as SkillProvider>::render_skill().expect("trait rendered");
    assert_eq!(rendered, trait_rendered);

    assert_clean_markdown(&rendered);
}

#[cfg(feature = "skill-minijinja")]
#[test]
fn test_minijinja_render_with_template_and_file_overrides() {
    // Render using an ad-hoc template string on FleetApp
    let custom_template =
        "# Fleet Skill Mini: {{ name }}\n\nCommands: {{ subcommands | length }}\n";
    let output =
        FleetApp::render_skill_with_template(custom_template).expect("rendered with template");
    let mut count = 4;
    if cfg!(feature = "mcp") {
        count += 1;
    }
    if cfg!(feature = "completions") {
        count += 1;
    }
    assert_eq!(
        output,
        format!("# Fleet Skill Mini: fleet\n\nCommands: {count}\n")
    );

    // Render using a template file
    let fixture_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/custom_skill.j2");
    let file_output = FleetApp::render_skill_with_template_file(&fixture_path)
        .expect("rendered with template file");
    assert!(file_output.contains(
        "# Custom Skill: fleet\n\nVersion: 2.1.0\nDescription: Fleet management CLI tool\n"
    ));

    // Render using MiniJinjaSkillRenderer::from_file directly
    let file_renderer =
        MiniJinjaSkillRenderer::from_file(&fixture_path).expect("renderer from file");
    let rendered = file_renderer
        .render(&FleetApp::cli_metadata())
        .expect("rendered");
    assert_eq!(rendered, file_output);

    // Non-existent template file fails gracefully with execution error
    let missing_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/does_not_exist.j2");
    let err = MiniJinjaSkillRenderer::from_file(&missing_path).expect_err("should fail");
    assert_eq!(err.kind(), clifacet::FailureKind::Execution);
    assert!(err.message().contains("cannot read template file"));
}

#[cfg(feature = "skill-minijinja")]
#[app(name = "minimal-tool", about = "Minimal zero-command CLI", skill)]
struct MinimalTool {}

#[cfg(feature = "skill-minijinja")]
#[test]
fn test_minijinja_minimal_cli_without_options_or_commands() {
    let rendered = MinimalTool::render_skill().expect("rendered minimal skill");

    assert!(
        rendered
            .starts_with("---\nname: minimal-tool\ndescription: Minimal zero-command CLI\n---\n\n")
    );
    assert!(rendered.contains("# minimal-tool\n\nMinimal zero-command CLI\n\n"));
    assert!(rendered.contains("## Non-Interactive Execution Rules\n\n"));
    assert!(!rendered.contains("## Options"));
    assert!(!rendered.contains("## Commands"));

    assert_clean_markdown(&rendered);
}

#[cfg(feature = "skill-minijinja")]
#[group(commands)]
pub struct NestedSubGroup {}

#[cfg(feature = "skill-minijinja")]
#[commands(group)]
impl NestedSubGroup {
    /// Ping nested service
    pub fn ping(&self) -> Result<String, Failure> {
        Ok("pong".into())
    }
}

#[cfg(feature = "skill-minijinja")]
#[group(commands)]
pub struct AdminGroup {}

#[cfg(feature = "skill-minijinja")]
#[commands(group)]
impl AdminGroup {
    /// Flush system cache
    pub fn flush(&self) -> Result<String, Failure> {
        Ok("flushed".into())
    }

    /// Access nested service subcommands
    pub fn service(&self, #[clifacet(subcommand)] sub: NestedSubGroup) -> Result<String, Failure> {
        sub.execute()
    }
}

#[cfg(feature = "skill-minijinja")]
#[app(
    name = "nest-app",
    about = "CLI with nested subcommand groups",
    skill,
    commands
)]
pub struct NestApp {}

#[cfg(feature = "skill-minijinja")]
#[commands]
impl NestApp {
    /// Root level status command
    pub fn status(&self) -> Result<String, Failure> {
        Ok("healthy".into())
    }

    /// Mount admin command group
    pub fn admin(&self, #[clifacet(subcommand)] admin: AdminGroup) -> Result<String, Failure> {
        admin.execute()
    }
}

#[cfg(feature = "skill-minijinja")]
#[test]
fn test_minijinja_nested_subcommand_hierarchy() {
    let rendered = NestApp::render_skill().expect("rendered nested skill");

    assert!(rendered.contains("# nest-app\n\nCLI with nested subcommand groups\n\n"));
    assert!(rendered.contains("### `nest-app status`\n\nRoot level status command\n"));
    assert!(rendered.contains("### `nest-app admin`\n\nMount admin command group\n"));
    assert!(rendered.contains("#### Subcommands\n\n"));
    assert!(rendered.contains("- `nest-app admin flush`: Flush system cache\n"));
    assert!(rendered.contains("- `nest-app admin service`: Access nested service subcommands\n"));

    assert_clean_markdown(&rendered);
}
