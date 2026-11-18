"""Automate version bump across repository manifests."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("release", "writes"),
    description="Bump repository version across Cargo, Pixi, and package manifests.",
)
def bump(ctx: ToolContext, target: str = "") -> None:
    """Run bump_version.py."""
    argv = [sys.executable, "tools/_lib/bump_version.py"]
    if target:
        argv.append(target)
    ctx.run(argv, cwd=ctx.repo_root)
