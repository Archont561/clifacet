"""Rust clippy lint tasks."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("ci", "rust", "offline"),
    description="Run clippy over the workspace, examples, and tests with warnings denied.",
)
def lint(ctx: ToolContext) -> None:
    """Verify Clippy lints across all targets and features."""
    ctx.run(
        [
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
        cwd=ctx.repo_root,
    )
