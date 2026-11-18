"""Repository structure and linting policy checks with alint."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "offline"),
    description="Run alint repository layout, schema, and policy checks.",
)
def alint(ctx: ToolContext) -> None:
    """Execute alint check with baseline and strict checking."""
    ctx.run(
        [
            "alint",
            "check",
            "--baseline",
            ".alint-baseline.json",
            "--strict-baseline",
            "--fail-on-warning",
        ],
        cwd=ctx.repo_root,
    )
