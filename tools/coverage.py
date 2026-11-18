"""Line coverage via cargo llvm-cov nextest."""

from __future__ import annotations

import sys
from pathlib import Path

from tasknest import ToolContext, tool


@tool(
    marks=("ci", "rust", "offline"),
    description="Measure line coverage and enforce coverage threshold.",
)
def coverage(ctx: ToolContext) -> None:
    """Run llvm-cov nextest and check_coverage.py."""
    cov_dir = ctx.repo_root / "coverage"
    cov_dir.mkdir(parents=True, exist_ok=True)
    report_path = cov_dir / "lcov.info"

    ctx.run(
        [
            "cargo",
            "llvm-cov",
            "nextest",
            "--workspace",
            "--all-features",
            "--locked",
            "--lcov",
            "--output-path",
            str(report_path),
        ],
        cwd=ctx.repo_root,
    )
    ctx.run([sys.executable, "tools/_lib/check_coverage.py", str(report_path)], cwd=ctx.repo_root)
