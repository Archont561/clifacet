"""Run pytest test suite over repository automation scripts."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "python", "offline"),
    description="Run pytest suite for repository Python automation scripts.",
)
def test_scripts(ctx: ToolContext) -> None:
    """Execute pytest tools/tests."""
    ctx.run([sys.executable, "-m", "pytest", "tools/tests"], cwd=ctx.repo_root)

__test__ = False
