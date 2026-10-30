#!/usr/bin/env python3
"""Prove the declared minimum supported Rust version by checking it in CI.

The declaration lives in `[workspace.package] rust-version`, so this script reads
it from the manifest instead of accepting a second, drift-prone copy. It then
checks every feature boundary with that toolchain, at every target, against the
committed lockfile.

A dependency that starts requiring a newer compiler makes `cargo` refuse to build
with "package X requires rustc Y", which fails this script: the advertised floor
cannot silently rot. Raising the floor is then a deliberate manifest change.
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

# `default` is excluded: every check runs with `--no-default-features`, so the
# default set is the empty combination rather than a combination of its own.
NON_FEATURE_KEYS = frozenset({"default"})


@dataclass(frozen=True)
class Plan:
    """One `cargo check` invocation, kept as data so it can be asserted in tests."""

    toolchain: str
    arguments: tuple[str, ...]

    def command(self) -> list[str]:
        """The argv to execute, with the toolchain selector when one is given."""
        return ["cargo", f"+{self.toolchain}", *self.arguments] if self.toolchain else ["cargo", *self.arguments]


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


def declared_msrv(manifest: dict, source: Path = WORKSPACE_MANIFEST) -> str:
    """Return `[workspace.package] rust-version`, the single source of the claim."""
    version = manifest.get("workspace", {}).get("package", {}).get("rust-version")
    if not isinstance(version, str) or not version.strip():
        raise ValueError(f"{source} declares no [workspace.package] rust-version")
    return version.strip()


def package_name(manifest: dict, source: Path = CRATE_MANIFEST) -> str:
    """Return the feature-bearing package name."""
    name = manifest.get("package", {}).get("name")
    if not isinstance(name, str) or not name:
        raise ValueError(f"{source} declares no package name")
    return name


def feature_sets(manifest: dict) -> list[list[str]]:
    """Every feature boundary to check: the empty set, each feature, and unset combinations.

    Checking features one at a time proves each optional dependency boundary on
    its own; the caller adds an all-features run for the combined boundary.
    """
    names = sorted(name for name in manifest.get("features", {}) if name not in NON_FEATURE_KEYS)
    return [[], *[[name] for name in names]]


def build_plan(toolchain: str, package: str, features: list[list[str]]) -> list[Plan]:
    """Plan one per-feature check plus a workspace-wide all-features check."""
    plan = [
        Plan(
            toolchain,
            (
                "check",
                "--locked",
                "--all-targets",
                "-p",
                package,
                "--no-default-features",
                *(() if not set_ else ("--features", ",".join(set_))),
            ),
        )
        for set_ in features
    ]
    plan.append(Plan(toolchain, ("check", "--locked", "--all-targets", "--all-features")))
    return plan


def execute(command: list[str]) -> subprocess.CompletedProcess:
    """Run one step, capturing output so a failure can be explained."""
    return subprocess.run(command, capture_output=True, text=True, check=False)


def run_plan(plan: Sequence[Plan], runner=execute) -> list[Failure]:
    """Execute each step in order, stopping at the first failure, and return it."""
    for index, step in enumerate(plan, start=1):
        command = step.command()
        print(f"[{index}/{len(plan)}] {' '.join(command)}", flush=True)
        result = runner(command)
        output = f"{result.stdout or ''}{result.stderr or ''}"
        if output:
            print(output, end="" if output.endswith("\n") else "\n", flush=True)
        if result.returncode != 0:
            return [Failure(step, result.returncode, output)]
    return []


def failure_hint(failure: Failure) -> str:
    """Name the likeliest cause, since the two causes need different fixes."""
    selected = any(argument.startswith("+") for argument in failure.plan.command())
    if selected and ("no such command" in failure.output or "not installed" in failure.output):
        return (
            f"rustup is required to select a toolchain, and rust {failure.plan.toolchain} is not "
            f"available here. Install it with `rustup toolchain install {failure.plan.toolchain}`."
        )
    return (
        "a dependency now needs a newer compiler than the declared rust-version, or this toolchain "
        "does not support the code. Raise `[workspace.package] rust-version` deliberately (with the "
        "scan of `rust-version` fields in the dependency graph), or pin the dependency down."
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--toolchain",
        help="Rust toolchain to check with; defaults to the declared rust-version",
    )
    parser.add_argument(
        "--print-toolchain",
        action="store_true",
        help="print the declared toolchain and exit, so CI can install exactly that",
    )
    parser.add_argument(
        "--print-plan",
        action="store_true",
        help="print the planned commands and exit without running them",
    )
    arguments = parser.parse_args(argv)

    try:
        workspace = load_manifest(WORKSPACE_MANIFEST)
        crate = load_manifest(CRATE_MANIFEST)
        toolchain = declared_msrv(workspace) if arguments.toolchain is None else arguments.toolchain
        if arguments.print_toolchain:
            print(toolchain)
            return 0
        plan = build_plan(toolchain, package_name(crate), feature_sets(crate))
    except ValueError as error:
        print(f"prove-msrv: {error}", file=sys.stderr)
        return 1

    if arguments.print_plan:
        for step in plan:
            print(" ".join(step.command()))
        return 0

    print(f"prove-msrv: checking {len(plan)} configuration(s) with rust {toolchain}")
    failed = run_plan(plan)
    if failed:
        failure = failed[0]
        print(
            f"prove-msrv: `{' '.join(failure.plan.command())}` failed with exit "
            f"{failure.returncode}.\nprove-msrv: {failure_hint(failure)}",
            file=sys.stderr,
        )
        return 1
    selected = f"rust {toolchain}" if toolchain else "the ambient cargo"
    print(f"prove-msrv: {selected} compiles every checked configuration")
    return 0


if __name__ == "__main__":
    sys.exit(main())
