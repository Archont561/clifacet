"""Rust test suite tasks via nextest."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("ci", "rust", "offline"),
    description="Run library and integration tests across the workspace via cargo nextest.",
)
def test(ctx: ToolContext) -> None:
    """Execute cargo nextest across the workspace."""
    ctx.run(
        ["cargo", "nextest", "run", "--workspace", "--all-features", "--locked"],
        cwd=ctx.repo_root,
    )
