"""Render or verify repository README.md."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("docs",),
    description="Render README.md from template or verify it matches the template.",
)
def readme(ctx: ToolContext, check: bool = False) -> None:
    """Run readme.py to render or check README.md."""
    argv = [sys.executable, "tools/_lib/readme.py"]
    if check:
        argv.append("--check")
    ctx.run(argv, cwd=ctx.repo_root)
