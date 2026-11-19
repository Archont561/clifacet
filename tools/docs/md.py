"""Lint Markdown and MDX documentation with rumdl."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "docs", "offline"),
    description="Lint and format-check Markdown and MDX via rumdl.",
)
def md(ctx: ToolContext, fix: bool = False) -> None:
    """Execute `rumdl check --deny-config-warnings` across the repository."""
    argv = ["rumdl", "check", "--deny-config-warnings"]
    if fix:
        argv.append("--fix")
    argv.append(".")
    ctx.run(argv, cwd=ctx.repo_root)
