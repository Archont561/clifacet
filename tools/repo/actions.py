"""GitHub Actions workflow linting with actionlint."""

from __future__ import annotations

from tasknest import ToolContext, tool


@tool(
    marks=("fast", "ci", "offline"),
    description="Lint workflow files with actionlint and shellcheck.",
)
def actions(ctx: ToolContext) -> None:
    """Run actionlint across all repository workflow files."""
    workflows = sorted(
        str(path.relative_to(ctx.repo_root))
        for path in (ctx.repo_root / ".github" / "workflows").glob("*.yml")
    )
    if workflows:
        ctx.run(["actionlint", *workflows], cwd=ctx.repo_root)
