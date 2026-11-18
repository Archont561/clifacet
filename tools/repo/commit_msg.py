"""Conventional commit message verification."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "offline"),
    description="Check conventional commit messages using convco.",
)
def commit_msg(ctx: ToolContext) -> None:
    """Validate commit message passed via stdin with convco."""
    ctx.run(["convco", "check", "--from-stdin", "--strip"], cwd=ctx.repo_root)
