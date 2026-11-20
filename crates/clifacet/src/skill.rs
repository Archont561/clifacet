//! Pluggable skill rendering for agent documentation (CLI-74, decision-25).

use crate::failure::Failure;
use crate::schema::CliMetadata;
use std::rc::Rc;
use std::sync::Arc;

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
}
