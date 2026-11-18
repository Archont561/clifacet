"""Composite offline repository quality gate."""

from __future__ import annotations

from tasknest import tool


@tool(
    depends=(
        "rust.fmt",
        "rust.lint",
        "repo.alint",
        "docs.prose",
        "docs.check",
        "docs.md",
        "release.changelog",
        "repo.skills",
        "rust.features",
        "repo.actions",
        "security.advisory",
        "rust.packages",
        "repo.sandbox",
        "test",
    ),
    marks=("ci", "offline"),
    description="Run all offline quality gates (formatting, lints, policy, docs, and tests).",
)
def check() -> None:
    """Execute all offline quality checks declared in `depends`."""
