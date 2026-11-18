"""Agent skills integrity checks."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "offline"),
    description="Verify agent skills and their lockfile integrity.",
)
def skills(ctx: ToolContext) -> None:
    """Run check_skills.py against .agents/skills and skills-lock.json."""
    ctx.run([sys.executable, "tools/_lib/check_skills.py"], cwd=ctx.repo_root)
