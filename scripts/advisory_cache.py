#!/usr/bin/env python3
"""Maintain the RustSec advisory-database cache the audit job restores.

`cargo audit` clones RustSec unless a database is already there, so CI caches one. That
cache has two pieces of logic, both formerly in workflow YAML:

- **the key** is an ISO year and week, with a `restore-keys` prefix so a run reuses last
  week's clone instead of re-downloading. The year in the key is the ISO week-year, which
  is not the calendar year: 2027-01-01 is still week 53 of 2026, and using `%Y` there
  gives two different keys for the same week.
- **the cleanup** resets whatever was restored. A clone left dirty by a cancelled run
  makes the next audit fail for a reason that has nothing to do with the code, which is
  a confusing way to learn about a cache.

`clean` is a no-op when nothing was restored, which is the cache-miss case and must not
be an error.
"""

from __future__ import annotations

import argparse
import datetime
import os
import subprocess
import sys
from collections.abc import Callable, Iterable, Sequence
from pathlib import Path

# Run as `python scripts/advisory_cache.py`, Python puts *this* directory on the import
# path rather than the repository root, so the `scripts.*` import below would fail.
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

Runner = Callable[[Sequence[str]], object]


def cache_key(today: datetime.date | None = None) -> str:
    """`<ISO year>-W<ISO week>`, the key a weekly cache wants."""
    date = today or datetime.datetime.now(datetime.UTC).date()
    iso = date.isocalendar()
    return f"{iso.year}-W{iso.week:02d}"


def clean(paths: Iterable[Path], run: Runner | None = None) -> list[Path]:
    """Reset and clean every restored clone; return the ones that were touched."""
    execute = run or _run
    cleaned = []
    for path in paths:
        if not (path / ".git").is_dir():
            continue
        execute(["git", "-C", str(path), "reset", "--hard", "HEAD"])
        execute(["git", "-C", str(path), "clean", "-ffdx"])
        cleaned.append(path)
    return cleaned


def default_paths(cargo_home: Path | None = None, home: Path | None = None) -> list[Path]:
    """Both layouts a cached database may be in: a single clone, or one per audit tool.

    A restored sandbox redirects `CARGO_HOME`, so `main` passes it in; the function
    itself reads no environment, which is what lets a test say where to look.
    """
    cargo = cargo_home if cargo_home is not None else (home or Path.home()) / ".cargo"
    paths = [cargo / "advisory-db"]
    nested = cargo / "advisory-dbs"
    if nested.is_dir():
        paths.extend(sorted(child for child in nested.iterdir() if child.is_dir()))
    return paths


def main(
    argv: Sequence[str] | None = None,
    today: datetime.date | None = None,
    output: Path | str | None = None,
    run: Runner | None = None,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("action", choices=["key", "clean"])
    parser.add_argument(
        "--cargo-home",
        type=Path,
        help="where the cached databases live; defaults to $CARGO_HOME or ~/.cargo",
    )
    arguments = parser.parse_args(argv)

    if arguments.action == "key":
        from scripts.github_actions import set_output

        key = cache_key(today)
        set_output("week", key, path=output)
        return 0

    cargo_home = arguments.cargo_home
    if cargo_home is None:
        redirected = os.environ.get("CARGO_HOME")
        cargo_home = Path(redirected) if redirected else None
    cleaned = clean(default_paths(cargo_home=cargo_home), run=run)
    if cleaned:
        print(f"advisory-cache: reset {len(cleaned)} restored clone(s): "
              f"{', '.join(path.name for path in cleaned)}")
    else:
        print("advisory-cache: no restored advisory database to clean")
    return 0


def _run(argv: Sequence[str]) -> object:
    return subprocess.run(list(argv), check=False, capture_output=True)


if __name__ == "__main__":
    sys.exit(main())
