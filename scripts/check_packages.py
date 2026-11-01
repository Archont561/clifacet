#!/usr/bin/env python3
"""Check what cargo would publish, without publishing it.

`release.yml` verifies the version sites and the changelog notes before it publishes,
but nothing looked at the tarball. That is the part a release cannot take back: a crate
published without its licence files, or with test scaffolding it never meant to ship,
stays that way for that version's lifetime.

This is the rehearsal half of the publication contract. It runs `cargo package --list`
for each crate — the same listing `cargo publish` uploads, resolved offline from the
lockfile — and checks two things:

- **contents**: the files a registry page needs are there (manifest, source root, README,
  both licence files) and repository-only paths are not (BDD `.feature` files, the
  sandbox and its vendored sources, the build directory);
- **metadata**: the fields a crates.io page shows are declared, resolved through
  `workspace = true` inheritance, because a missing description cannot be corrected
  after the version is indexed.

The publish order is fixed here too: the macro crate first, then the main crate, which
cannot resolve `clifacet-macros = "<version>"` until that version exists on the index.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Publish order, not alphabetical: the main crate's registry dependency on the macro
# crate cannot resolve until that exact version is published.
CRATES = ("clifacet-macros", "clifacet")

REQUIRED_FILES = ("Cargo.toml", "README.md", "LICENSE-MIT", "LICENSE-APACHE", "src/lib.rs")
# Paths that belong to the repository and its tooling, never to a published crate.
FORBIDDEN_PREFIXES = (".pixi-sandbox/", "node_modules/", "target/", ".git/")
FORBIDDEN_SUFFIXES = (".feature",)
METADATA = ("description", "license", "repository", "rust-version")

Runner = Callable[[Sequence[str], Path], str]


@dataclass(frozen=True)
class Finding:
    """One reason a crate is not ready to publish, and which crate it is."""

    kind: str
    detail: str

    def __str__(self) -> str:
        return f"{self.kind}: {self.detail}"


def package_files(crate: str, root: Path = ROOT, run: Runner | None = None) -> list[str]:
    """The files `cargo package` would upload for `crate`, as relative paths."""
    # `--allow-dirty` so this also runs on a working tree mid-change. In CI the tree is
    # clean, so the listing is exactly the committed package; locally any extra line is
    # an untracked file that would ship, which is worth seeing.
    command = ["cargo", "package", "--list", "--locked", "-p", crate, "--allow-dirty"]
    output = (run or _run)(command, root)
    # The listing is one relative path per line; anything indented is cargo's own
    # progress chatter, and blank lines are spacing.
    return [line for line in output.splitlines() if line and line == line.strip()]


def _run(command: Sequence[str], root: Path) -> str:
    try:
        completed = subprocess.run(
            command, cwd=root, capture_output=True, text=True, check=True
        )
    except FileNotFoundError as error:
        raise ValueError(f"cargo is not available: {error}") from error
    except subprocess.CalledProcessError as error:
        raise ValueError(
            f"`{' '.join(command)}` failed: {error.stderr.strip() or error.stdout.strip()}"
        ) from error
    return completed.stdout


def contents(crate: str, files: Sequence[str]) -> list[Finding]:
    """The content rules for one crate's tarball."""
    missing = [name for name in REQUIRED_FILES if name not in files]
    forbidden = [name for name in files if _repository_only(name)]

    findings = []
    if missing:
        findings.append(
            Finding("missing-file", f"{crate} does not package: {', '.join(missing)}")
        )
    if forbidden:
        findings.append(
            Finding(
                "forbidden-file",
                f"{crate} would package repository-only paths: {', '.join(forbidden)}",
            )
        )
    return findings


def _repository_only(name: str) -> bool:
    return name.startswith(FORBIDDEN_PREFIXES) or name.endswith(FORBIDDEN_SUFFIXES)


def manifest(root: Path, crate: str) -> tuple[dict, dict]:
    """The crate's `[package]` table and the workspace `[workspace.package]` table."""
    package = _table(root / "crates" / crate / "Cargo.toml", "package")
    workspace = _table(root / "Cargo.toml", "workspace", "package")
    return package, workspace


def _table(path: Path, *keys: str) -> dict:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    for key in keys:
        data = data.get(key, {})
    return data


def resolve_metadata(package: dict, workspace: dict) -> dict:
    """The metadata values a crate declares, with `workspace = true` followed through."""
    resolved = {}
    for key in METADATA:
        if key not in package:
            continue
        value = package[key]
        if isinstance(value, dict) and value.get("workspace") is True:
            value = workspace.get(key)
        if value is not None:
            resolved[key] = value
    return resolved


def metadata(crate: str, package: dict, workspace: dict) -> list[Finding]:
    """The registry-page fields a published crate must declare."""
    missing = [key for key in METADATA if key not in package]
    unresolvable = [
        key
        for key in METADATA
        if key in package and isinstance(package[key], dict) and key not in resolve_metadata(package, workspace)
    ]

    findings = []
    if missing:
        findings.append(
            Finding("missing-metadata", f"{crate} declares no {', '.join(missing)}")
        )
    if unresolvable:
        findings.append(
            Finding(
                "unresolvable-metadata",
                f"{crate} inherits {', '.join(unresolvable)} from the workspace, "
                f"which does not define it",
            )
        )
    return findings


def check(crates: Sequence[str] = CRATES, root: Path = ROOT, run: Runner | None = None) -> list[Finding]:
    """Every finding about `crates`, in publish order."""
    findings = []
    for crate in crates:
        files = package_files(crate, root=root, run=run)
        package, workspace = manifest(root, crate)
        findings.extend(contents(crate, files))
        findings.extend(metadata(crate, package, workspace))
    return findings


def main(argv: Sequence[str] | None = None, root: Path = ROOT) -> int:
    """Check the crates; a finding is a reason to fix the package, not to publish it."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--list",
        action="store_true",
        help="print each crate's packaged file count and exit",
    )
    arguments = parser.parse_args(argv)

    findings = check(root=root)
    if findings:
        for finding in findings:
            print(f"check-packages: {finding}", file=sys.stderr)
        return 1

    if arguments.list:
        for crate in CRATES:
            print(f"{crate}: {len(package_files(crate, root=root))} files")
    print(f"check-packages: {', '.join(CRATES)} package only what a release should ship")
    return 0


if __name__ == "__main__":
    sys.exit(main())
