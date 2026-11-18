"""Lefthook git hooks installation."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("writes", "offline"),
    description="Install configured lefthook git hooks into .git/hooks.",
)
def hooks(ctx: ToolContext) -> None:
    """Run lefthook install."""
    ctx.run(["lefthook", "install"], cwd=ctx.repo_root)
