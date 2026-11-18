"""Verify Rust feature matrix combinations."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("ci", "rust", "offline"),
    description="Compile every supported feature combination, alone and together.",
)
def features(ctx: ToolContext) -> None:
    """Run check_features.py across crate feature boundaries."""
    ctx.run([sys.executable, "tools/_lib/check_features.py"], cwd=ctx.repo_root)
