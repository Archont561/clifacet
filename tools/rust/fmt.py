"""Rust formatting tasks."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "rust", "offline"),
    description="Format Rust sources or verify formatting with cargo fmt.",
)
def fmt(ctx: ToolContext, check: bool = False) -> None:
    """Format Rust code with cargo fmt or verify formatting."""
    argv = ["cargo", "fmt", "--all"]
    if check:
        argv.extend(["--", "--check"])
    ctx.run(argv, cwd=ctx.repo_root)
