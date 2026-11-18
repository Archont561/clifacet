"""Prove declared minimum supported Rust version."""

from __future__ import annotations

import sys

from tasknest import ToolContext, tool


@tool(
    marks=("rust", "offline"),
    description="Check feature boundaries against the compiler declared as the floor.",
)
def msrv(ctx: ToolContext, toolchain: str = "1.88") -> None:
    """Run prove_msrv.py with the declared or ambient toolchain."""
    ctx.run(
        [sys.executable, "tools/_lib/prove_msrv.py", "--toolchain", toolchain],
        cwd=ctx.repo_root,
    )
