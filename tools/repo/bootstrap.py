"""Bootstrap repository environment and prove it."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("writes",),
    description="Reach remote head, restore environment, install hooks, and prove it.",
)
def bootstrap(ctx: ToolContext) -> None:
    """Run scripts/bootstrap.sh."""
    ctx.run(["bash", "scripts/bootstrap.sh"], cwd=ctx.repo_root)
