#!/usr/bin/env python3
"""Record a release version in every manifest that carries it.

The version lives in three places, and `release.yml` fails if they disagree:

- `Cargo.toml` `[workspace.package] version`, which both crates inherit
- `Cargo.toml` `[workspace.dependencies] clifacet-macros`, the pinned requirement
- `pixi.toml` `[workspace] version`, the environment version

Bumping them by hand is how a release drifts, so one command rewrites all three, and
`--check` verifies they agree without writing anything — which is what CI and
`release.yml` use.

The changelog is deliberately *not* here. It is rendered from the conventional commits by
`scripts/changelog.py`, so a bump cannot disagree with the history it claims to describe:
the version sites say which version is next, and the commits say what it contains.
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import tomllib
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO_MANIFEST = ROOT / "Cargo.toml"
PIXI_MANIFEST = ROOT / "pixi.toml"

# Semver as Cargo accepts it: three numeric parts, optional pre-release and build.
SEMVER = re.compile(
    r"^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
    r"(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$"
)


@dataclass(frozen=True)
class Site:
    """One place the version is recorded, named so a mismatch can be reported."""

    label: str
    value: str


def parse_version(value: str) -> str:
    """Validate a release version, since Cargo rejects anything else at publish time."""
    candidate = value.strip()
    if not SEMVER.fullmatch(candidate):
        raise ValueError(f"'{value}' is not a semver version such as 0.2.0 or 1.0.0-rc.1")
    return candidate


def _section(lines: list[str], header: str) -> tuple[int, int]:
    """Return the half-open line range of a TOML section, excluding its header."""
    start = None
    for index, line in enumerate(lines):
        stripped = line.strip()
        if not stripped.startswith("["):
            continue
        if start is not None:
            return start, index
        if stripped == header:
            start = index + 1
    if start is None:
        raise ValueError(f"no {header} section")
    return start, len(lines)


def _rewrite_section(text: str, header: str, pattern: re.Pattern[str], value: str) -> str:
    """Replace the capture group of the first `pattern` match inside one section."""
    lines = text.splitlines(keepends=True)
    start, end = _section(lines, header)
    for index in range(start, end):
        match = pattern.search(lines[index])
        if match is None:
            continue
        line = lines[index]
        lines[index] = line[: match.start(1)] + value + line[match.end(1):]
        return "".join(lines)
    raise ValueError(f"{header} has no line matching {pattern.pattern}")


def _read_section(text: str, header: str, pattern: re.Pattern[str]) -> str:
    """Read the capture group of the first `pattern` match inside one section."""
    lines = text.splitlines()
    start, end = _section(lines, header)
    for line in lines[start:end]:
        match = pattern.search(line)
        if match is not None:
            return match.group(1)
    raise ValueError(f"{header} has no line matching {pattern.pattern}")


WORKSPACE_VERSION = re.compile(r'^version\s*=\s*"([^"]*)"\s*$')
MACRO_REQUIREMENT = re.compile(r'^clifacet-macros\s*=\s*\{[^}]*version\s*=\s*"([^"]*)"')


def versions(cargo: str, pixi: str) -> list[Site]:
    """Read the three recorded versions."""
    return [
        Site("Cargo.toml [workspace.package] version", _read_section(cargo, "[workspace.package]", WORKSPACE_VERSION)),
        Site("Cargo.toml [workspace.dependencies] clifacet-macros", _read_section(cargo, "[workspace.dependencies]", MACRO_REQUIREMENT)),
        Site("pixi.toml [workspace] version", _read_section(pixi, "[workspace]", WORKSPACE_VERSION)),
    ]


def inconsistencies(cargo: str, pixi: str) -> list[str]:
    """Report every site whose version differs from the workspace version."""
    sites = versions(cargo, pixi)
    expected = sites[0].value
    return [
        f"{site.label} is '{site.value}' but Cargo.toml [workspace.package] version is '{expected}'"
        for site in sites[1:]
        if site.value != expected
    ]


def bump(cargo: str, pixi: str, version: str) -> tuple[str, str]:
    """Rewrite every version site to `version`, leaving other lines untouched."""
    cargo = _rewrite_section(cargo, "[workspace.package]", WORKSPACE_VERSION, version)
    cargo = _rewrite_section(cargo, "[workspace.dependencies]", MACRO_REQUIREMENT, version)
    pixi = _rewrite_section(pixi, "[workspace]", WORKSPACE_VERSION, version)
    return cargo, pixi


def crate_pins(root: Path, crates: Sequence[str]) -> list[str]:
    """Crates that declare their own `version` instead of inheriting the workspace one.

    Both published crates use `version.workspace = true`, so the workspace table is the
    only place a release has to read. A crate that pins a version is not a syntax error —
    it is a silently different release, which is why the release job refuses it.
    """
    problems = []
    for crate in crates:
        manifest = root / "crates" / crate / "Cargo.toml"
        try:
            package = tomllib.loads(manifest.read_text(encoding="utf-8")).get("package", {})
        except (OSError, tomllib.TOMLDecodeError) as error:
            problems.append(f"{manifest} cannot be read: {error}")
            continue
        declared = package.get("version")
        if declared is None:
            continue
        if isinstance(declared, dict) and declared.get("workspace") is True:
            continue
        problems.append(
            f"{manifest} pins version {declared} instead of inheriting the workspace version"
        )
    return problems


def relock() -> int:
    """Refresh Cargo.lock offline; a version bump needs no network or new crates."""
    cargo = shutil.which("cargo")
    if cargo is None:
        print(
            "bump-version: cargo is not on PATH, so Cargo.lock was not refreshed. "
            "Run the bump through Pixi (`pixi run --frozen --no-install bump-version ...`) "
            "or run `cargo metadata --offline` yourself.",
            file=sys.stderr,
        )
        return 1
    try:
        result = subprocess.run(
            [cargo, "metadata", "--offline", "--format-version", "1"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        print(f"bump-version: could not run cargo: {error}", file=sys.stderr)
        return 1
    if result.returncode != 0:
        print(f"bump-version: relock failed:\n{result.stderr}", file=sys.stderr)
    return result.returncode


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("version", nargs="?", help="new version to record, for example 0.2.0")
    parser.add_argument("--print-version", action="store_true", help="print the current version and exit")
    parser.add_argument("--check", action="store_true", help="fail if the recorded versions disagree")
    parser.add_argument("--dry-run", action="store_true", help="show the change without writing it")
    parser.add_argument("--relock", action="store_true", help="refresh Cargo.lock after writing")
    arguments = parser.parse_args(argv)

    try:
        cargo = CARGO_MANIFEST.read_text(encoding="utf-8")
        pixi = PIXI_MANIFEST.read_text(encoding="utf-8")
        current = versions(cargo, pixi)[0].value

        if arguments.print_version:
            print(current)
            return 0
        if arguments.check:
            problems = inconsistencies(cargo, pixi)
            for problem in problems:
                print(f"bump-version: {problem}", file=sys.stderr)
            if problems:
                return 1
            print(f"bump-version: all three version sites record {current}")
            return 0
        if arguments.version is None:
            parser.error("a version is required unless --check or --print-version is used")

        version = parse_version(arguments.version)
        if version == current:
            raise ValueError(f"{version} is already the recorded version")
        new_cargo, new_pixi = bump(cargo, pixi, version)
    except ValueError as error:
        print(f"bump-version: {error}", file=sys.stderr)
        return 1

    if arguments.dry_run:
        print(f"bump-version: would record {version} (was {current}) in:")
        for site in versions(new_cargo, new_pixi):
            print(f"  {site.label} = {site.value}")
        return 0

    CARGO_MANIFEST.write_text(new_cargo, encoding="utf-8")
    PIXI_MANIFEST.write_text(new_pixi, encoding="utf-8")
    print(f"bump-version: recorded {version} (was {current})")
    if arguments.relock:
        if relock() != 0:
            return 1
        print("bump-version: Cargo.lock refreshed")
    else:
        print("bump-version: run `cargo check` (no --locked) or `pixi run bump-relock` to refresh Cargo.lock")
    return 0


if __name__ == "__main__":
    sys.exit(main())
