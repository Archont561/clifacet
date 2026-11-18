"""Check unmaintained advisory exception validity."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "security", "offline"),
    description="Verify advisory exception is active, valid, and not expired.",
)
def advisory(ctx: ToolContext) -> None:
    """Run check_advisory_exception.py."""
    ctx.run([sys.executable, "tools/_lib/check_advisory_exception.py"], cwd=ctx.repo_root)
