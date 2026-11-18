"""Documentation and snippet verification."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "docs", "offline"),
    description="Verify documentation links, anchors, and compiled code snippets.",
)
def check(ctx: ToolContext, write: bool = False) -> None:
    """Run check_docs.py to verify documentation integrity."""
    argv = [sys.executable, "tools/_lib/check_docs.py"]
    if write:
        argv.append("--write")
    ctx.run(argv, cwd=ctx.repo_root)
