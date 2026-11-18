"""Pixi sandbox policy and launcher checks."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "offline"),
    description="Verify restore launcher and workflows match pixi-sandbox.toml policy.",
)
def sandbox(ctx: ToolContext, check: bool = True) -> None:
    """Run pixi-sandbox init --check or plan."""
    argv = [
        "pixi-sandbox",
        "init",
        "--config",
        "pixi-sandbox.toml",
        "--script-path",
        "scripts/restore.sh",
    ]
    if check:
        argv.append("--check")
    ctx.run(argv, cwd=ctx.repo_root)
