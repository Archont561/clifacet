"""Rust documentation tests."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("ci", "rust", "docs", "offline"),
    description="Run documentation doctests across the workspace.",
)
def doc(ctx: ToolContext) -> None:
    """Execute cargo test --doc across the workspace."""
    ctx.run(
        ["cargo", "test", "--workspace", "--all-features", "--doc", "--locked"],
        cwd=ctx.repo_root,
    )
