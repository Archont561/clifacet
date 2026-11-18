"""Verify cargo package tarball contents."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("ci", "rust", "offline"),
    description="Check cargo publish tarball contents and exclude scaffolding.",
)
def packages(ctx: ToolContext) -> None:
    """Run check_packages.py against cargo package lists."""
    ctx.run([sys.executable, "tools/_lib/check_packages.py"], cwd=ctx.repo_root)
