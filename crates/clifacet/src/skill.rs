//! Pluggable skill rendering for agent documentation (CLI-74, decision-25).

use crate::failure::Failure;
use crate::schema::CliMetadata;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(feature = "skill-minijinja")]
pub const DEFAULT_SKILL_TEMPLATE: &str = include_str!("../templates/skill.md.j2");

/// A skill renderer that uses `MiniJinja` to render agent documentation templates (CLI-75, decision-25).
#[cfg(feature = "skill-minijinja")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniJinjaSkillRenderer {
    template: Option<String>,
}

#[cfg(feature = "skill-minijinja")]
impl Default for MiniJinjaSkillRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "skill-minijinja")]
impl MiniJinjaSkillRenderer {
    /// Creates a new `MiniJinjaSkillRenderer` using the default embedded template.
    #[must_use]
    pub fn new() -> Self {
        Self { template: None }
    }

    /// Creates a new `MiniJinjaSkillRenderer` with a custom template string.
    #[must_use]
    pub fn with_template(template: impl Into<String>) -> Self {
        Self {
            template: Some(template.into()),
        }
    }

    /// Creates a new `MiniJinjaSkillRenderer` reading a custom template from a file path.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if reading the file fails.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, Failure> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            Failure::execution(format!(
                "cannot read template file `{}`: {e}",
                path.display()
            ))
        })?;
        Ok(Self::with_template(content))
    }

    /// Returns the template string used by this renderer.
    #[must_use]
    pub fn template(&self) -> &str {
        self.template.as_deref().unwrap_or(DEFAULT_SKILL_TEMPLATE)
    }
}

#[cfg(feature = "skill-minijinja")]
impl SkillRenderer for MiniJinjaSkillRenderer {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure> {
        let template_source = self.template();
        let mut env = minijinja::Environment::new();

        env.add_template("skill.md.j2", template_source)
            .map_err(|e| Failure::execution(format!("failed to parse skill template: {e}")))?;
        let tmpl = env
            .get_template("skill.md.j2")
            .map_err(|e| Failure::execution(format!("failed to load skill template: {e}")))?;
        let raw = tmpl
            .render(minijinja::value::Serde(cli))
            .map_err(|e| Failure::execution(format!("failed to render skill template: {e}")))?;

        Ok(clean_rendered_markdown(&raw))
    }
}

#[cfg(feature = "skill-minijinja")]
fn clean_rendered_markdown(raw: &str) -> String {
    let mut cleaned_lines: Vec<&str> = Vec::new();
    let mut prev_blank = false;

    for line in raw.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if !prev_blank {
                cleaned_lines.push("");
                prev_blank = true;
            }
        } else {
            cleaned_lines.push(trimmed);
            prev_blank = false;
        }
    }

    while let Some(&"") = cleaned_lines.first() {
        cleaned_lines.remove(0);
    }
    while let Some(&"") = cleaned_lines.last() {
        cleaned_lines.pop();
    }

    let mut result = cleaned_lines.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }
    result
}

/// A pluggable renderer converting [`CliMetadata`] into agent documentation or skill content.
pub trait SkillRenderer {
    /// Renders the CLI metadata into a formatted string (such as Markdown or JSON).
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if rendering fails.
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure>;
}

impl SkillRenderer for fn(&CliMetadata) -> Result<String, Failure> {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure> {
        self(cli)
    }
}

impl<T: SkillRenderer + ?Sized> SkillRenderer for &T {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure> {
        (**self).render(cli)
    }
}

impl<T: SkillRenderer + ?Sized> SkillRenderer for Box<T> {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure> {
        (**self).render(cli)
    }
}

impl<T: SkillRenderer + ?Sized> SkillRenderer for Arc<T> {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure> {
        (**self).render(cli)
    }
}

impl<T: SkillRenderer + ?Sized> SkillRenderer for Rc<T> {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure> {
        (**self).render(cli)
    }
}

/// A trait for CLI types configured with a [`SkillRenderer`].
pub trait SkillProvider {
    /// The skill renderer type.
    type Renderer: SkillRenderer;

    /// Returns an instance of the configured renderer.
    fn skill_renderer() -> Self::Renderer;

    /// Renders the CLI metadata using the configured renderer.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if rendering fails.
    fn render_skill() -> Result<String, Failure>;

    /// Writes or checks the agent skill documentation according to the given options.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] if writing fails or drift is detected.
    fn write_skill(options: &WriteSkillOptions) -> Result<String, Failure>;
}

/// Options for writing or checking agent skill documentation (CLI-76, decision-25).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriteSkillOptions {
    /// Write to standard output instead of a file.
    pub stdout: bool,

    /// Explicit output file path. Defaults to `.agents/skills/<name>/SKILL.md`.
    pub output: Option<std::path::PathBuf>,

    /// Compare on-disk file with current rendering and fail on drift without writing.
    pub check: bool,
}

/// Executes the `write-skill` action for a CLI given its name, rendered markdown, and options.
///
/// # Errors
///
/// Returns a [`Failure`] if reading/writing fails or if drift is detected when `check` is enabled.
pub fn execute_write_skill(
    cli_name: &str,
    rendered: &str,
    options: &WriteSkillOptions,
) -> Result<String, Failure> {
    let path = options
        .output
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from(format!(".agents/skills/{cli_name}/SKILL.md")));

    if options.check {
        if !path.exists() {
            return Err(Failure::execution(format!(
                "skill document `{}` does not exist (drift detected)",
                path.display()
            )));
        }
        let existing = std::fs::read_to_string(&path).map_err(|e| {
            Failure::execution(format!(
                "cannot read skill document `{}`: {e}",
                path.display()
            ))
        })?;
        let existing_norm = existing.replace("\r\n", "\n");
        let rendered_norm = rendered.replace("\r\n", "\n");
        if existing_norm != rendered_norm {
            return Err(Failure::execution(format!(
                "skill document `{}` has drifted from current CLI definition",
                path.display()
            )));
        }
        return Ok(format!("skill document `{}` is up to date", path.display()));
    }

    if options.stdout {
        return Ok(rendered.to_string());
    }

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| {
            Failure::execution(format!(
                "cannot create directory `{}`: {e}",
                parent.display()
            ))
        })?;
    }

    std::fs::write(&path, rendered).map_err(|e| {
        Failure::execution(format!(
            "cannot write skill document `{}`: {e}",
            path.display()
        ))
    })?;

    Ok(format!("wrote skill document to `{}`", path.display()))
}
