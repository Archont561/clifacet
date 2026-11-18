"""Run release workflow steps."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("release",),
    description="Execute release preflight, packaging, reporting, and publication.",
)
def release(ctx: ToolContext, step: str = "preflight") -> None:
    """Run release.py with the specified step."""
    ctx.run([sys.executable, "tools/_lib/release.py", step], cwd=ctx.repo_root)
