"""Composite test suite."""

from __future__ import annotations

from tasknest import tool


@tool(
    depends=("rust.test", "rust.doc", "repo.test_scripts"),
    marks=("ci", "offline"),
    description="Run Rust tests, doctests, and Python script tests.",
)
def test() -> None:
    """Execute all test suites declared in `depends`."""
