"""Cargo security advisory audit using cargo-audit."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("security", "rust"),
    description="Audit dependencies for security advisories using cargo-audit.",
)
def audit(ctx: ToolContext, online: bool = False) -> None:
    """Run cargo audit against RustSec database or local offline database."""
    if online:
        ctx.run(["cargo", "audit", "--deny", "warnings"], cwd=ctx.repo_root)
    else:
        ctx.run([sys.executable, "tools/_lib/audit_local.py"], cwd=ctx.repo_root)
