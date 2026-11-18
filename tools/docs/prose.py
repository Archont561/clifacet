"""Lint technical documentation prose with Vale."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "docs", "offline"),
    description="Check committed Vale styles rendering and lint documentation prose.",
)
def prose(ctx: ToolContext, sync: bool = False) -> None:
    """Run prose check and lint or write and sync."""
    if sync:
        ctx.run([sys.executable, "tools/_lib/prose.py", "write"], cwd=ctx.repo_root)
        ctx.run([sys.executable, "tools/_lib/prose.py", "sync"], cwd=ctx.repo_root)
    else:
        ctx.run([sys.executable, "tools/_lib/prose.py", "check"], cwd=ctx.repo_root)
        ctx.run([sys.executable, "tools/_lib/prose.py", "lint"], cwd=ctx.repo_root)
