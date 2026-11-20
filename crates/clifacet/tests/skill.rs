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
