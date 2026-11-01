"""The GitHub Actions protocols, in one place instead of in workflow YAML.

A shell step in a workflow can hand a value to the next step and add to the run
summary; a Python script doing the same job needs those two file protocols, and needs
them to degrade sensibly when it is run by hand. Everything here is standard library
and file-based, so the workflows stay plumbing and the logic stays testable.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path


def set_output(name: str, value: str, path: Path | str | None = None) -> None:
    """Publish a step output.

    `name=value` is the common form, but it cannot carry a newline, so a multiline
    value uses the heredoc form GitHub also accepts. Without an output file — a local
    run — the value is printed, because silently dropping it would make the script
    untestable by hand.
    """
    target = _target(path, "GITHUB_OUTPUT")
    if target is None:
        print(f"{name}={value}")
        return

    if "\n" in value:
        delimiter = f"EOF_{name}"
        block = f"{name}<<{delimiter}\n{value}\n{delimiter}\n"
    else:
        block = f"{name}={value}\n"
    with target.open("a", encoding="utf-8") as handle:
        handle.write(block)


def append_summary(markdown: str, path: Path | str | None = None) -> None:
    """Add a section to the run summary, separated by one blank line."""
    target = _target(path, "GITHUB_STEP_SUMMARY")
    if target is None:
        print(markdown)
        return
    with target.open("a", encoding="utf-8") as handle:
        handle.write(f"{markdown.rstrip()}\n\n")


def set_error(message: str) -> None:
    """Annotate the run with an error, in the syntax the runner renders."""
    print(f"::error::{message}", file=sys.stderr)


def _target(path: Path | str | None, variable: str) -> Path | None:
    if path is not None:
        return Path(path)
    location = os.environ.get(variable)
    return Path(location) if location else None
