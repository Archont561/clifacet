#!/usr/bin/env python3
"""Check the RustSec advisory exception against the lockfile, and say when to drop it.

`basic-toml` is unmaintained (RUSTSEC-2026-0374) and reachable from `rstest-bdd`,
which is a test dependency. The project keeps `rstest-bdd` and ignores that one
advisory in `deny.toml` and `.cargo/audit.toml` — an exception by advisory ID across
the dependency graph, precisely because it is *not* a guarantee that the crate can
only be reached from tests.

An exception with no expiry is a decision nobody revisits, so this script gives it
one. It reads `Cargo.lock` and reports the three states that need a human:

- **missing-exception** — the crate is still in the graph but one of the two files no
  longer ignores the advisory, so a gate will fail somewhere else, later;
- **stale-exception** — the crate is gone, so both exceptions can be deleted; this is
  the signal the review asked for, and it fails the build on purpose;
- **documented-path** — `docs/development/development.md` quotes a dependency path that
  `Cargo.lock` no longer agrees with, which is a review that has quietly expired.

The path is rendered from the lockfile rather than written by hand, so the
documentation cannot drift from the graph it describes.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCKFILE = ROOT / "Cargo.lock"
DENY = ROOT / "deny.toml"
AUDIT = ROOT / ".cargo" / "audit.toml"
# Relative to the tree being checked, not to this file: the checks take a root so a
# fixture can be a temporary tree, and an absolute constant would read the real one.
DOCS = Path("docs") / "development" / "development.md"

ADVISORY = "RUSTSEC-2026-0374"
SECTION = "## Dependency advisory exception"
# The top of the path: the crate that pulls the unmaintained one in, transitively.
CRATE = "rstest-bdd"
TARGET = "basic-toml"
SEPARATOR = " → "

PACKAGE = re.compile(r"^\[\[package\]\]$")
NAME = re.compile(r'^name = "(.*)"$')
VERSION = re.compile(r'^version = "(.*)"$')
DEPENDENCY = re.compile(r'^"(.*)",$')
# The guide quotes the chain as `name version` pairs joined by the separator. Requiring a
# version on every hop is what separates a dependency path from prose: the release
# section contains lines like "Actions → release → Run workflow", and those are not paths.
PATH_LINE = re.compile(rf"^[a-z0-9_.-]+ \d[^\s]*(?:{SEPARATOR}[a-z0-9_.-]+ \d[^\s]*)+$")


@dataclass(frozen=True)
class Finding:
    """One problem with the exception, and what to do about it."""

    kind: str
    detail: str

    def __str__(self) -> str:
        return f"{self.kind}: {self.detail}"


def load_lock(path: Path = LOCKFILE) -> dict:
    """Read the package table out of a `Cargo.lock` without a TOML dependency.

    The repository has no TOML writer and the lockfile's shape here is three keys, so a
    line reader is both sufficient and immune to the ordering `cargo` chooses.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise ValueError(f"cannot read {path}: {error}") from error

    packages: list[dict] = []
    current: dict | None = None
    in_dependencies = False
    for line in text.splitlines():
        stripped = line.strip()
        if PACKAGE.match(stripped):
            current = {"name": "", "version": "", "dependencies": []}
            packages.append(current)
            in_dependencies = False
            continue
        if current is None:
            continue
        if (match := NAME.match(stripped)) is not None:
            current["name"] = match.group(1)
            in_dependencies = False
        elif (match := VERSION.match(stripped)) is not None:
            current["version"] = match.group(1)
            in_dependencies = False
        elif stripped == "dependencies = [":
            in_dependencies = True
        elif in_dependencies and (match := DEPENDENCY.match(stripped)) is not None:
            # A lockfile dependency is `name` or `name version`; only the name is needed.
            current["dependencies"].append(match.group(1).split(" ")[0])
        elif stripped == "]":
            in_dependencies = False
    return {"package": [package for package in packages if package["name"]]}


def dependency_path(lock: dict, start: str, target: str) -> list[tuple[str, str]] | None:
    """The shortest chain from `start` to `target`, as `(name, version)` pairs.

    Breadth-first, because the first path found is the one a reader can follow, and
    because a cycle in the graph must not turn into an infinite walk.
    """
    versions = {package["name"]: package.get("version", "") for package in lock["package"]}
    if start not in versions or target not in versions:
        return None

    edges = {package["name"]: package.get("dependencies", []) for package in lock["package"]}
    queue: list[list[str]] = [[start]]
    seen = {start}
    while queue:
        chain = queue.pop(0)
        for neighbour in edges.get(chain[-1], []):
            if neighbour in seen:
                continue
            if neighbour == target:
                return [(name, versions.get(name, "")) for name in (*chain, neighbour)]
            seen.add(neighbour)
            queue.append([*chain, neighbour])
    return None


def render_path(path: list[tuple[str, str]] | None) -> str:
    """The chain as one line, so the documentation can quote exactly what the lock says."""
    if not path:
        return ""
    return SEPARATOR.join(f"{name} {version}".strip() for name, version in path)


def documented_path(text: str) -> str:
    """The path the guide quotes in its advisory section, or an empty string.

    Scoped to the section and matched on shape, not on the presence of the separator:
    the same file describes release steps with arrows, and picking the first line that
    contains one made the check report its own documentation.
    """
    for line in _section(text, SECTION).splitlines():
        stripped = line.strip()
        if PATH_LINE.match(stripped):
            return stripped
    return ""


def _section(text: str, heading: str) -> str:
    """The body of `heading`, up to the next heading of the same or higher level."""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.strip() == heading:
            level = len(heading) - len(heading.lstrip("#"))
            body = []
            for candidate in lines[index + 1:]:
                stripped = candidate.strip()
                if stripped.startswith("#") and len(stripped) - len(stripped.lstrip("#")) <= level:
                    break
                body.append(candidate)
            return "\n".join(body)
    return ""


def exceptions(deny: str, audit: str) -> dict[str, set[str]]:
    """The advisory IDs each file ignores; comments are not exceptions."""
    return {"deny": _ignored(deny), "audit": _ignored(audit)}


def _ignored(text: str) -> set[str]:
    """The advisory IDs an `ignore = [...]` list names, in either file's style.

    The list can be one line (`ignore = ["RUSTSEC-…"]`) or a table array
    (`ignore = [{ id = "RUSTSEC-…", reason = "…" }]`), so the block is read until the
    closing bracket rather than assumed to fit on one line. A commented-out entry is a
    note, not an exception.
    """
    ids: set[str] = set()
    collecting = False
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if not collecting:
            if not stripped.startswith("ignore"):
                continue
            collecting = True
        ids.update(re.findall(r"RUSTSEC-\d{4}-\d{4}", stripped))
        if "]" in stripped:
            collecting = False
    return ids


def check(root: Path = ROOT) -> list[Finding]:
    """Every finding about the exception, most important first."""
    lock = load_lock(root / LOCKFILE.name)
    path = dependency_path(lock, CRATE, TARGET)
    ignored = exceptions(
        (root / DENY.name).read_text(encoding="utf-8"),
        (root / AUDIT.parent.name / AUDIT.name).read_text(encoding="utf-8"),
    )

    if path is None:
        return [
            Finding(
                "stale-exception",
                f"{TARGET} is no longer reachable from {CRATE}, so {ADVISORY} can be removed "
                f"from deny.toml and .cargo/audit.toml, and this script with it",
            )
        ]

    findings = []
    for name, pathname in (("deny", DENY.name), ("audit", f"{AUDIT.parent.name}/{AUDIT.name}")):
        if ADVISORY not in ignored[name]:
            findings.append(
                Finding(
                    "missing-exception",
                    f"{pathname} does not ignore {ADVISORY}, but {TARGET} "
                    f"({render_path(path)}) is still in the graph",
                )
            )

    quoted = documented_path((root / DOCS).read_text(encoding="utf-8"))
    actual = render_path(path)
    if quoted != actual:
        findings.append(
            Finding(
                "documented-path",
                f"docs/development/development.md quotes '{quoted or '(nothing)'}' but Cargo.lock says "
                f"'{actual}'; update the review in that section",
            )
        )
    return findings


def main(argv: Sequence[str] | None = None, root: Path = ROOT) -> int:
    """Drive the check; `--print-path` answers "what does the lock actually say?"."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--print-path",
        action="store_true",
        help="print the dependency path as one line and exit",
    )
    arguments = parser.parse_args(argv)

    lock = load_lock(root / LOCKFILE.name)
    path = dependency_path(lock, CRATE, TARGET)

    if arguments.print_path:
        print(render_path(path) if path else f"{TARGET} is not in the graph")
        return 0

    findings = check(root)
    if findings:
        for finding in findings:
            print(f"check-advisory: {finding}", file=sys.stderr)
        return 1
    print(f"check-advisory: {ADVISORY} is still warranted — {render_path(path)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
