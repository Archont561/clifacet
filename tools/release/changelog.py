"""Render or verify CHANGELOG.md from conventional commits."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "release", "offline"),
    description="Render CHANGELOG.md from conventional commits or verify consistency.",
)
def changelog(ctx: ToolContext, check: bool = False) -> None:
    """Run changelog.py check or regen."""
    action = "check" if check else "regen"
    ctx.run([sys.executable, "tools/_lib/changelog.py", action], cwd=ctx.repo_root)
