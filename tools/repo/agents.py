"""Refresh generated regions of AGENTS.md."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("writes", "offline"),
    description="Refresh generated regions of AGENTS.md with @eugeniughelbur/agents-md.",
)
def agents(ctx: ToolContext) -> None:
    """Run agents-md to refresh generated sections in AGENTS.md."""
    ctx.run(["bun", "run", "--silent", "agents-md", "--", "--no-symlink"], cwd=ctx.repo_root)
