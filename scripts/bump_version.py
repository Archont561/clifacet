#!/usr/bin/env python3
"""Bump the release version across every manifest that records it, and read notes.

The version lives in three places, and `release.yml` fails if they disagree:

- `Cargo.toml` `[workspace.package] version`, which both crates inherit
- `Cargo.toml` `[workspace.dependencies] clifacet-macros`, the pinned requirement
- `pixi.toml` `[workspace] version`, the environment version

Bumping them by hand is how a release drifts, so one command rewrites all three,
promotes the changelog's `Unreleased` section to the new version, and leaves the
relock to `cargo`. `--check` verifies the three agree without writing anything,
which is what CI and `release.yml` use.

An application starting at its first version has nothing to bump to: the version
sites already record it, so `--promote-current` dates the `Unreleased` section
without touching them.
"""

from __future__ import annotations

import argparse
import datetime
import re
import shutil
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO_MANIFEST = ROOT / "Cargo.toml"
PIXI_MANIFEST = ROOT / "pixi.toml"
CHANGELOG = ROOT / "CHANGELOG.md"
REPOSITORY = "https://github.com/Archont561/clifacet"

# Semver as Cargo accepts it: three numeric parts, optional pre-release and build.
SEMVER = re.compile(
    r"^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
    r"(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$"
)

RELEASE_HEADING = "## [Unreleased]"


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


LINK_DEFINITION = re.compile(r"^\[[^\]]+\]:\s")


def _sections(text: str) -> list[tuple[str, list[str]]]:
    """Split a changelog into `##` sections, ignoring trailing link definitions.

    Keep a Changelog puts `[version]: url` references after the last section, so
    they must not leak into that section's notes.
    """
    sections: list[tuple[str, list[str]]] = []
    for line in text.splitlines():
        if line.startswith("## "):
            sections.append((line, []))
        elif sections and not LINK_DEFINITION.match(line):
            sections[-1][1].append(line)
    return sections


def changelog_notes(text: str, version: str) -> str:
    """Return the body of the changelog section for `version`."""
    for heading, body in _sections(text):
        if heading == f"## [{version}]" or heading.startswith(f"## [{version}] "):
            notes = "\n".join(body).strip()
            if not notes:
                raise ValueError(f"the changelog section for {version} is empty")
            return notes + "\n"
    known = ", ".join(heading.split()[1].strip("[]") for heading, _ in _sections(text)) or "none"
    raise ValueError(f"CHANGELOG.md has no '## [{version}]' section (found: {known})")


def promote_changelog(text: str, version: str, date: str) -> str:
    """Turn the `Unreleased` section into the released version and reopen it.

    Entries are required, so a bump cannot quietly ship an empty changelog.
    """
    if f"## [{version}]" in text:
        raise ValueError(f"CHANGELOG.md already has a '## [{version}]' section")
    headings = [heading for heading, _ in _sections(text)]
    if RELEASE_HEADING not in headings:
        raise ValueError(f"CHANGELOG.md has no '{RELEASE_HEADING}' section")
    for heading, body in _sections(text):
        if heading != RELEASE_HEADING:
            continue
        if not "\n".join(body).strip():
            raise ValueError(f"'{RELEASE_HEADING}' is empty; describe the change before releasing")
        break
    text = text.replace(RELEASE_HEADING, f"## [{version}] - {date}", 1)
    # Reopen an empty Unreleased section directly above the released one.
    released = f"## [{version}] - {date}"
    text = text.replace(released, f"{RELEASE_HEADING}\n\n{released}", 1)
    return _update_links(text, version)


def _update_links(text: str, version: str) -> str:
    """Point the changelog's link references at the version that just shipped."""
    current = "[Unreleased]: "
    if current not in text:
        return text
    tag_line = f"[{version}]: {REPOSITORY}/releases/tag/v{version}"
    if tag_line in text:
        return text
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.startswith(current):
            lines[index] = f"[Unreleased]: {REPOSITORY}/compare/v{version}...HEAD"
            lines.insert(index + 1, tag_line)
            break
    return "\n".join(lines) + "\n"


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
    parser.add_argument("--notes", metavar="VERSION", help="print the changelog section for VERSION")
    parser.add_argument(
        "--promote-current",
        action="store_true",
        help="date the Unreleased section as the recorded version, without bumping",
    )
    parser.add_argument("--dry-run", action="store_true", help="show the change without writing it")
    parser.add_argument("--skip-changelog", action="store_true", help="do not promote the changelog")
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
        if arguments.notes is not None:
            if not CHANGELOG.is_file():
                raise ValueError("CHANGELOG.md is missing")
            sys.stdout.write(changelog_notes(CHANGELOG.read_text(encoding="utf-8"), arguments.notes))
            return 0
        if arguments.promote_current:
            # The first release of an application: the version is already recorded, so
            # the only thing missing is the dated section that `--notes` reads. The
            # manifests are left alone on purpose — this is not a bump.
            if not CHANGELOG.is_file():
                raise ValueError("CHANGELOG.md is missing")
            today = datetime.datetime.now(datetime.UTC).date().isoformat()
            promoted = promote_changelog(CHANGELOG.read_text(encoding="utf-8"), current, today)
            if arguments.dry_run:
                print(f"bump-version: would promote {RELEASE_HEADING} to '## [{current}] - {today}'")
                return 0
            CHANGELOG.write_text(promoted, encoding="utf-8")
            print(f"bump-version: promoted {RELEASE_HEADING} to '## [{current}] - {today}'")
            return 0
        if arguments.version is None:
            parser.error(
                "a version is required unless --check, --print-version, --notes or --promote-current is used"
            )

        version = parse_version(arguments.version)
        if version == current:
            raise ValueError(f"{version} is already the recorded version")
        new_cargo, new_pixi = bump(cargo, pixi, version)
        new_changelog = None
        if not arguments.skip_changelog:
            if not CHANGELOG.is_file():
                raise ValueError("CHANGELOG.md is missing; pass --skip-changelog to release without notes")
            new_changelog = promote_changelog(
                CHANGELOG.read_text(encoding="utf-8"),
                version,
                datetime.datetime.now(datetime.UTC).date().isoformat(),
            )
    except ValueError as error:
        print(f"bump-version: {error}", file=sys.stderr)
        return 1

    if arguments.dry_run:
        print(f"bump-version: would record {version} (was {current}) in:")
        for site in versions(new_cargo, new_pixi):
            print(f"  {site.label} = {site.value}")
        print("bump-version: would promote the changelog" if new_changelog else "bump-version: changelog untouched")
        return 0

    CARGO_MANIFEST.write_text(new_cargo, encoding="utf-8")
    PIXI_MANIFEST.write_text(new_pixi, encoding="utf-8")
    if new_changelog is not None:
        CHANGELOG.write_text(new_changelog, encoding="utf-8")
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
