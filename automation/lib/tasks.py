"""The bridge between a doit task and a script that already knows how to run.

Every script in `scripts/` is a `main(argv) -> int` with an `if __name__ == "__main__"`
guard, which is what doit wants: importing the module runs nothing, and calling `main` is
the whole task. This wraps that in the two things doit cannot infer:

- **The exit code is a failure.** doit treats a Python action's return value as the task's
  result, so a `main` that reports a problem by returning 1 would be a passed task. A
  non-zero return raises instead, which is the only way doit sees it.
- **The arguments are never ambiguous.** A `main` that takes `argv` is always given one,
  even an empty one, because an argument of `None` makes `argparse` read `sys.argv` — and
  under doit, `sys.argv` is doit's own command line. That mistake is silent and confusing:
  the task fails with an argument error naming a subcommand the script never had.

A script whose `main` takes no `argv` is called without one; the signature is read rather
than assumed, so both kinds of script are driven the same way from `dodo.py`.
"""

from __future__ import annotations

import importlib
import inspect
from collections.abc import Sequence

SCRIPTS = "scripts"


def main(name: str):
    """`scripts/<name>.py`'s `main`."""
    return importlib.import_module(f"{SCRIPTS}.{name}").main


def run(name: str, argv: Sequence[str] | None = None) -> None:
    """Run `scripts/<name>.py`'s `main`, failing the task when it returns non-zero."""
    entry = main(name)
    takes_argv = "argv" in inspect.signature(entry).parameters
    code = entry(list(argv or [])) if takes_argv else entry()
    if code:
        message = f"scripts/{name}.py exited {code}"
        raise RuntimeError(message)
