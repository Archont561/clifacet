#!/usr/bin/env python3
"""Check the conventional-commit contract for exactly the commits this run introduced.

`.versionrc` derives the changelog, the version bump and the release notes from commit
messages, and the hook that checks them is local: anyone without hooks, or anyone who
skips them, can land a message that breaks the next release. The `commit-messages` job
is the enforceable half of that contract.

The job used to resolve its range in YAML — an `if`/`elif`/`else` writing a step output,
then a second step branching on whether it was empty, because the runner's actionlint
rejects the unquoted expansion a one-line form would need. That decision is now here,
where it can be exercised without pushing anything.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from collections.abc import Callable, Mapping, Sequence

DEFAULT_DEPTH = 20
# git's "no such commit": the before-SHA of a branch's first push.
NO_BASE = "0" * 40

Runner = Callable[[Sequence[str]], int]


def resolve_range(event: str, pr_base: str = "", push_base: str = "") -> str:
    """The range to check, or an empty string when there is no base to compare against."""
    if event == "pull_request":
        return f"{pr_base}..HEAD"
    if push_base and push_base != NO_BASE:
        return f"{push_base}..HEAD"
    return ""


def command(range_: str, depth: int = DEFAULT_DEPTH) -> list[str]:
    """One invocation, quoted, so the runner's shellcheck has nothing to object to."""
    if range_:
        return ["convco", "check", range_]
    return ["convco", "check", "-n", str(depth)]


def check(event: str, pr_base: str = "", push_base: str = "") -> list[str]:
    """The command this event needs; separated from running it so tests can read it."""
    return command(resolve_range(event, pr_base, push_base))


def main(
    argv: Sequence[str] | None = None,
    env: Mapping[str, str] | None = None,
    run: Runner | None = None,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "action",
        nargs="?",
        default="check",
        choices=["check", "resolve"],
        help="check runs convco; resolve prints the command without running it",
    )
    arguments = parser.parse_args(argv)

    environment = os.environ if env is None else env
    invocation = check(
        environment.get("EVENT", ""),
        environment.get("PR_BASE", ""),
        environment.get("PUSH_BASE", ""),
    )

    if arguments.action == "resolve":
        print(" ".join(invocation))
        return 0

    execute = run or _run
    try:
        return execute(invocation)
    except FileNotFoundError:
        print(
            "commit-range: convco is not on PATH. Run this through Pixi "
            "(`pixi run --frozen commit-range`), which is what the workflow does.",
            file=sys.stderr,
        )
        return 1


def _run(argv: Sequence[str]) -> int:
    return subprocess.run(list(argv), check=False).returncode


if __name__ == "__main__":
    sys.exit(main())
