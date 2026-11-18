"""Lint Markdown files across the repository."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "docs", "offline"),
    description="Lint Markdown structure and syntax via markdownlint-cli2.",
)
def md(ctx: ToolContext) -> None:
    """Run markdownlint-cli2 across documentation and markdown files."""
    ctx.run(["bun", "run", "markdownlint-cli2"], cwd=ctx.repo_root)
