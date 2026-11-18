"""Cargo dependency verification using cargo-deny."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("security", "rust"),
    description="Run cargo-deny checks (bans, licenses, sources, and advisories).",
)
def deny(ctx: ToolContext, online: bool = False) -> None:
    """Run cargo-deny check (online) or offline subset (bans, licenses, sources)."""
    if online:
        ctx.run(["cargo", "deny", "check"], cwd=ctx.repo_root)
    else:
        ctx.run(["cargo", "deny", "check", "bans", "licenses", "sources"], cwd=ctx.repo_root)
