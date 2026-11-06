#!/usr/bin/env python3
"""Decide whether a change is documentation only, and so which lane of CI it takes.

A change that touches nothing but documentation cannot break the Rust build, so it runs
the checks documentation can break (`alint`, the skills, Backlog, guide and changelog
checks) instead of the full matrix. Everything else runs all of CI.

The decision has one safe direction, and every doubtful case takes it: an unrecognised
path, an empty change, a dispatch, a branch's first push and a diff git cannot compute
all run everything. The range is the one the commit-message check uses
(`commit_range.resolve_range`), so both jobs mean the same thing by "this change".

Documentation is decided by path, not by content, which is only sound while no Rust
source compiles a Markdown file in; `scripts/tests/test_change_scope.py` fails the day
one does. A guide's `rust` snippets are compiled from a generated test file, so a change
to a snippet is a change to that file too, and takes the full lane.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from collections.abc import Callable, Mapping, Sequence
from pathlib import Path, PurePosixPath

# Run as `python3 scripts/change_scope.py`, Python puts *this* directory on the import
# path rather than the repository root, so the `scripts.*` imports below would fail.
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.commit_range import resolve_range  # noqa: E402  (needs the path above)
from scripts.github_actions import append_summary, set_output  # noqa: E402

# The step output the workflow's job conditions read.
DOCS_ONLY = "docs-only"

DOCUMENTATION_DIRECTORIES = ("backlog", "docs")
DOCUMENTATION_SUFFIX = ".md"
# Only the repository's own licenses: one inside a crate is part of what it publishes.
ROOT_DOCUMENT_PREFIX = "LICENSE"

# `git`, as (exit code, stdout); separated so the tests can stand in for it.
Git = Callable[[Sequence[str]], tuple[int, str]]


def is_documentation(path: str) -> bool:
    """Whether a repository path is documentation, by where it lives and what it is."""
    parts = PurePosixPath(path).parts
    if not parts:
        return False
    if parts[0] in DOCUMENTATION_DIRECTORIES and len(parts) > 1:
        return True
    if len(parts) == 1 and parts[0].startswith(ROOT_DOCUMENT_PREFIX):
        return True
    return path.endswith(DOCUMENTATION_SUFFIX)


def documentation_only(paths: Sequence[str]) -> bool:
    """Whether a change is documentation and nothing else. An empty change is not."""
    return bool(paths) and all(is_documentation(path) for path in paths)


def diff_command(range_: str) -> list[str]:
    """The changed paths in a range: NUL-separated, because Backlog file names have
    spaces, and without rename detection, so a file moved out of the code still counts."""
    return ["git", "diff", "--name-only", "-z", "--no-renames", range_]


def changed_paths(range_: str, git: Git) -> tuple[list[str] | None, str]:
    """The paths a range changes, or `None` and the reason they are unknown."""
    if not range_:
        return None, "there is no base commit to compare against"
    try:
        code, output = git(diff_command(range_))
    except FileNotFoundError:
        return None, "git is not installed"
    if code != 0:
        return None, f"git could not compute the diff for `{range_}` (exit {code})"
    return [path for path in output.split("\0") if path], ""


def summary(docs_only: bool, paths: list[str] | None, reason: str) -> str:
    """The run-summary section: the lane, and the evidence for it."""
    if docs_only:
        heading = "**Documentation only**: the full Rust jobs are skipped."
    else:
        heading = "**Full CI**: every job runs."
    lines = ["### Change scope", "", heading]
    if reason:
        lines += ["", f"Not classified, because {reason}."]
    if paths:
        code = [path for path in paths if not is_documentation(path)]
        shown = code or paths
        label = "Not documentation" if code else "Changed"
        lines += ["", f"{label}:", ""]
        lines += [f"- `{path}`" for path in shown[:50]]
        if len(shown) > 50:
            lines.append(f"- … and {len(shown) - 50} more")
    return "\n".join(lines)


def main(
    argv: Sequence[str] | None = None,
    env: Mapping[str, str] | None = None,
    git: Git | None = None,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.parse_args(argv)

    environment = os.environ if env is None else env
    range_ = resolve_range(
        environment.get("EVENT", ""),
        environment.get("PR_BASE", ""),
        environment.get("PUSH_BASE", ""),
    )
    paths, reason = changed_paths(range_, git or _git)
    docs_only = paths is not None and documentation_only(paths)

    set_output(DOCS_ONLY, "true" if docs_only else "false", environment.get("GITHUB_OUTPUT"))
    append_summary(summary(docs_only, paths, reason), environment.get("GITHUB_STEP_SUMMARY"))
    # Never a failure: a change that cannot be classified runs everything, which is safe.
    return 0


def _git(argv: Sequence[str]) -> tuple[int, str]:
    completed = subprocess.run(list(argv), capture_output=True, text=True, check=False)
    return completed.returncode, completed.stdout


if __name__ == "__main__":
    sys.exit(main())
