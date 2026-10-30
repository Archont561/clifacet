"""Run cargo-audit only where a local advisory database can answer it.

`cargo audit` fetches the RustSec database unless `--no-fetch` is given, and
`--no-fetch` without a local clone fails outright. A pre-push hook therefore
cannot simply call `cargo audit`: on a restored sandbox, or any host without a
cached database, every push would be blocked by a check that CI already runs
with the network available.

This script makes that distinction explicit. It runs the audit against the local
database when one exists, and otherwise says so and steps aside — unless
`--require` is passed, which turns the absence into a failure for callers (CI)
that must never audit nothing silently.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def database_path(cargo_home: str | None, home: Path) -> Path:
    """Where cargo-audit looks for the advisory database.

    cargo-audit defaults to `~/.cargo/advisory-db`, but a restored sandbox
    redirects `CARGO_HOME`, so the environment wins when it is set.
    """
    if cargo_home:
        return Path(cargo_home) / "advisory-db"
    return home / ".cargo" / "advisory-db"


def database_ready(path: Path) -> bool:
    """A usable clone has the advisory `crates/` index; an empty dir does not."""
    return (path / "crates").is_dir()


def decide(ready: bool, require: bool) -> str:
    """Return `run`, `skip`, or `fail` for the audit step."""
    if ready:
        return "run"
    return "fail" if require else "skip"


def audit_command(database: Path) -> list[str]:
    """`--stale` keeps a week-old clone from failing a `--deny warnings` run."""
    return ["cargo", "audit", "--db", str(database), "--no-fetch", "--stale", "--deny", "warnings"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--require",
        action="store_true",
        help="fail instead of skipping when no local advisory database exists",
    )
    args = parser.parse_args(argv)

    database = database_path(os.environ.get("CARGO_HOME"), Path.home())
    ready = database_ready(database)
    action = decide(ready, args.require)

    if action == "skip":
        print(
            "audit: no local advisory database at "
            f"{database}, so this audit is skipped.\n"
            "audit: CI runs `cargo audit --deny warnings` with the database available; "
            "to audit here, run `cargo audit` once with network access.",
            file=sys.stderr,
        )
        return 0
    if action == "fail":
        print(
            f"audit: no local advisory database at {database} and --require was given.\n"
            "audit: run `cargo audit` once with network access to clone it.",
            file=sys.stderr,
        )
        return 1

    result = subprocess.run(audit_command(database), cwd=ROOT, check=False)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
