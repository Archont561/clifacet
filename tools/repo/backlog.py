"""Backlog card status and dependency graph validation."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("fast",),
    description="Validate backlog task graph and check card completion status.",
)
def backlog(ctx: ToolContext) -> None:
    """Execute backlog doctor and check_backlog_status.py."""
    ctx.run(["bun", "x", "--bun", "--no-install", "backlog", "doctor"], cwd=ctx.repo_root)
    ctx.run([sys.executable, "tools/_lib/check_backlog_status.py"], cwd=ctx.repo_root)
