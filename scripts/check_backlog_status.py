#!/usr/bin/env python3
"""Check that a task's status agrees with its own acceptance criteria.

`backlog doctor` validates the graph: duplicate ids, missing dependencies, cycles. It has
nothing to say about whether a card's status is *true*, and the tooling never will —
`definition_of_done` adds items to a new task, it does not compare them to the status
later. So a card whose criteria are all checked sat in `In Progress` for hours, and a card
with one criterion open sat in `Done`, and both read as finished work to anyone scanning
the board.

Two rules, because there are two ways to disagree:

- a task in `Done` with an unchecked criterion is claiming more than it proved;
- a task in any other status with every criterion checked has finished and not said so.

The second is the milder fault and the more common one, and it is not a spelling mistake:
it is the moment someone has to decide whether the last criterion was really met. The
message names the task and the count, and leaves the decision where it belongs.

A task with no criteria at all is not checked. Plenty of cards are a note to self, and
inventing a rule for them would make this guard about the tool rather than the work.
"""

from __future__ import annotations

import re
import sys
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TASKS = ROOT / "backlog" / "tasks"

STATUS = re.compile(r"^status:\s*(?P<status>.+?)\s*$", re.MULTILINE)
CRITERION = re.compile(r"^- \[(?P<mark>[ xX])\] #\d+", re.MULTILINE)
AC_BLOCK = re.compile(r"<!-- AC:BEGIN -->(?P<body>.*?)<!-- AC:END -->", re.DOTALL)

DONE = "Done"


@dataclass(frozen=True)
class Task:
    """One card, reduced to the two facts this guard compares."""

    path: Path
    status: str
    met: int
    total: int

    @property
    def identifier(self) -> str:
        """`cli-49` from `cli-49 - Release-0.4.0.md`, so a message names the board's id."""
        return self.path.name.split(" - ", 1)[0].upper()

    @property
    def finished(self) -> bool:
        """Whether every criterion is met. A card with none is never "finished"."""
        return self.total > 0 and self.met == self.total


def parse(path: Path) -> Task | None:
    """Read a card's status and criterion marks; `None` when it is not a card at all.

    Only the acceptance-criteria block counts. A checked box in the notes is prose about
    the work, not a claim about done, and counting it would let a task pass by mentioning
    a tick.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        return None
    status = STATUS.search(text)
    if status is None:
        return None
    block = AC_BLOCK.search(text)
    body = block.group("body") if block else ""
    marks = [match.group("mark").lower() for match in CRITERION.finditer(body)]
    return Task(
        path=path,
        status=status.group("status"),
        met=sum(1 for mark in marks if mark == "x"),
        total=len(marks),
    )


def load(directory: Path = TASKS) -> list[Task]:
    """Every card in the backlog, in filename order, so failures are reported stably."""
    tasks = (parse(path) for path in sorted(directory.glob("*.md")))
    return [task for task in tasks if task is not None]


def problems(tasks: Iterable[Task]) -> list[str]:
    """One sentence per card whose status and criteria disagree."""
    found = []
    for task in tasks:
        if task.status == DONE and task.met < task.total:
            found.append(
                f"{task.identifier} is Done with {task.total - task.met} of {task.total} "
                "acceptance criteria unchecked"
            )
        elif task.status != DONE and task.finished:
            found.append(
                f"{task.identifier} has every acceptance criterion met but is "
                f"{task.status!r}"
            )
    return found


def main(argv: Sequence[str] | None = None, directory: Path | None = None) -> int:
    """Report the cards that disagree, and fail when any do.

    `directory` exists so a test can point the check at a tree it made, rather than at the
    repository's own backlog — which is the one tree that must always pass.
    """
    parser = _parser()
    arguments = parser.parse_args(argv)
    tasks = load(directory or TASKS)
    found = problems(tasks)

    if arguments.list:
        for task in tasks:
            print(f"{task.identifier}\t{task.status}\t{task.met}/{task.total}")
        return 0

    if found:
        for problem in found:
            print(f"backlog-status: {problem}", file=sys.stderr)
        print(
            "backlog-status: a status has to agree with the criteria. Fix the status if the "
            "work is finished; leave it and say why in the notes if the last criterion is "
            "not met yet.",
            file=sys.stderr,
        )
        return 1

    claimed = sum(1 for task in tasks if task.finished)
    print(f"backlog-status: {len(tasks)} task(s) agree with their criteria ({claimed} complete)")
    return 0


def _parser():
    import argparse

    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--list",
        action="store_true",
        help="print every task's status and criterion count, and exit",
    )
    return parser


if __name__ == "__main__":
    sys.exit(main())
