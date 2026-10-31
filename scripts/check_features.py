#!/usr/bin/env python3
"""Check every supported feature combination, and say which ones those are.

A feature is a dependency boundary: an application enables the frontends it uses
and pays for no others. Each boundary therefore has to compile *alone* — a crate
whose `tui` module needs something only `interactive` pulls in would still build
with `--all-features`, which is why "the tests pass" is not the claim here.

The claimed matrix is data, not a hand-written list of commands:

- `no-default-features` — nothing enabled at all;
- `default` — whatever the manifest's `default` set is, which is empty today and
  therefore the same configuration as the row above (the plan collapses them and
  names both, rather than running one command twice);
- one row per declared feature;
- `all-features` — every feature together, which is the configuration the
  compatibility claim is made against.

Deriving the rows from the manifest is what keeps the matrix from rotting: adding
a feature adds a row, and `prove_msrv.py` reuses this module, so the stable and
MSRV matrices cannot disagree about what is supported.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKSPACE_MANIFEST = ROOT / "Cargo.toml"
CRATE_MANIFEST = ROOT / "crates" / "clifacet" / "Cargo.toml"

# `default` is not a boundary of its own: it is whatever set the manifest names, so it
# gets a row of its own below rather than being treated as a feature like the others.
DEFAULT_FEATURE = "default"

NO_DEFAULT = "no-default-features"
ALL_FEATURES = "all-features"


@dataclass(frozen=True)
class Combination:
    """One row of the matrix: a name and the exact feature set it stands for.

    `features` is the *resolved* set, not the flags: it is what lets the plan notice
    that two rows describe one configuration and collapse them.
    """

    name: str
    features: tuple[str, ...] = ()
    all_features: bool = False

    def arguments(self, package: str) -> tuple[str, ...]:
        """The feature-related arguments this row adds to `cargo check`."""
        if self.all_features:
            if self.features:
                raise ValueError(
                    f"combination '{self.name}' claims all-features and also names a subset "
                    f"({', '.join(self.features)}); a row is one configuration or the other"
                )
            return ("--all-features",)
        if not self.features:
            return ("--no-default-features",)
        return ("--no-default-features", "--features", ",".join(self.features))


@dataclass(frozen=True)
class Plan:
    """One `cargo check` invocation, kept as data so it can be asserted in tests."""

    toolchain: str
    arguments: tuple[str, ...]
    covers: tuple[str, ...] = ()

    def command(self) -> list[str]:
        """The argv to execute, with the toolchain selector when one is given."""
        return [*self.cargo(), *self.arguments]

    def cargo(self) -> list[str]:
        """The cargo invocation, with the toolchain selector when one is given."""
        return ["cargo", f"+{self.toolchain}"] if self.toolchain else ["cargo"]


@dataclass(frozen=True)
class Failure:
    """A step that failed, with the output needed to explain why."""

    plan: Plan
    returncode: int
    output: str


def load_manifest(path: Path) -> dict:
    """Parse a Cargo manifest, or explain which file could not be read."""
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ValueError(f"cannot read {path}: {error}") from error
    except tomllib.TOMLDecodeError as error:
        raise ValueError(f"{path} is not valid TOML: {error}") from error


def package_name(manifest: dict, source: Path = CRATE_MANIFEST) -> str:
    """Return the feature-bearing package name."""
    name = manifest.get("package", {}).get("name")
    if not isinstance(name, str) or not name:
        raise ValueError(f"{source} declares no package name")
    return name


def declared_features(manifest: dict) -> list[str]:
    """Every feature the manifest declares except `default`, in a stable order."""
    features = manifest.get("features", {})
    return sorted(name for name in features if name != DEFAULT_FEATURE)


def default_features(manifest: dict) -> list[str]:
    """The `default` feature set, which may be empty and is empty today."""
    return [str(name) for name in manifest.get("features", {}).get(DEFAULT_FEATURE, [])]


def combinations(manifest: dict) -> list[Combination]:
    """The matrix this crate claims to support, derived from its own manifest."""
    features = declared_features(manifest)
    return [
        Combination(NO_DEFAULT),
        Combination(DEFAULT_FEATURE, tuple(default_features(manifest))),
        *(Combination(name, (name,)) for name in features),
        Combination(ALL_FEATURES, all_features=True),
    ]


def build_plan(toolchain: str, package: str, matrix: Sequence[Combination]) -> list[Plan]:
    """One `cargo check` per distinct configuration, naming every row it covers.

    Rows that resolve to the same feature set share a command: running the empty set
    twice would make the matrix look wider than it is, and hiding the duplicate would
    make the documented list and the plan disagree.
    """
    steps: list[Plan] = []
    index: dict[tuple[bool, tuple[str, ...]], int] = {}
    for combination in matrix:
        key = (combination.all_features, combination.features)
        if key in index:
            position = index[key]
            merged = steps[position]
            steps[position] = Plan(merged.toolchain, merged.arguments, (*merged.covers, combination.name))
            continue
        # The combined boundary is checked across the workspace, not one package: it is
        # the configuration a consumer builds, so every member has to compile in it.
        scope: tuple[str, ...] = () if combination.all_features else ("-p", package)
        arguments = ("check", "--locked", "--all-targets", *scope, *combination.arguments(package))
        index[key] = len(steps)
        steps.append(Plan(toolchain, arguments, (combination.name,)))
    return steps


def execute(command: list[str]) -> subprocess.CompletedProcess:
    """Run one step, capturing output so a failure can be explained."""
    return subprocess.run(command, capture_output=True, text=True, check=False)


def run_plan(plan: Sequence[Plan], runner=execute) -> list[Failure]:
    """Execute each step in order, stopping at the first failure, and return it."""
    for index, step in enumerate(plan, start=1):
        command = step.command()
        print(f"[{index}/{len(plan)}] {' '.join(command)}  ({', '.join(step.covers)})", flush=True)
        result = runner(command)
        output = f"{result.stdout or ''}{result.stderr or ''}"
        if output:
            print(output, end="" if output.endswith("\n") else "\n", flush=True)
        if result.returncode != 0:
            return [Failure(step, result.returncode, output)]
    return []


def first_error_line(output: str) -> str:
    """The line of cargo's output that names the problem, or the first non-empty one."""
    lines = [line.strip() for line in output.splitlines() if line.strip()]
    return next((line for line in lines if line.startswith("error")), lines[0] if lines else "")


def describe_failure(failure: Failure) -> str:
    """Name the row that broke, and quote what cargo said about it."""
    rows = ", ".join(failure.plan.covers) or "this configuration"
    detail = first_error_line(failure.output)
    return (
        f"`{' '.join(failure.plan.command())}` failed with exit {failure.returncode}, so "
        f"{rows} is not a configuration this crate supports. A feature has to compile "
        f"alone: an item that needs something another feature brings in must be gated "
        f"on that feature too."
        + (f"\ncargo said: {detail}" if detail else "")
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--toolchain",
        default="",
        help="Rust toolchain to check with, as `cargo +<toolchain>`; default: the ambient one",
    )
    parser.add_argument(
        "--print-matrix",
        action="store_true",
        help="print the supported combinations and exit without running them",
    )
    parser.add_argument(
        "--print-plan",
        action="store_true",
        help="print the commands the matrix resolves to and exit without running them",
    )
    arguments = parser.parse_args(argv)

    try:
        crate = load_manifest(CRATE_MANIFEST)
        matrix = combinations(crate)
        plan = build_plan(arguments.toolchain, package_name(crate), matrix)
    except ValueError as error:
        print(f"check-features: {error}", file=sys.stderr)
        return 1

    if arguments.print_matrix:
        for combination in matrix:
            rows = "all features" if combination.all_features else (", ".join(combination.features) or "none")
            print(f"{combination.name:<20} {rows}")
        return 0

    if arguments.print_plan:
        for step in plan:
            print(" ".join(step.command()))
        return 0

    print(
        f"check-features: {len(matrix)} supported combination(s), "
        f"{len(plan)} distinct configuration(s)"
    )
    failed = run_plan(plan)
    if failed:
        print(f"check-features: {describe_failure(failed[0])}", file=sys.stderr)
        return 1
    print("check-features: every supported combination compiles")
    return 0


if __name__ == "__main__":
    sys.exit(main())
