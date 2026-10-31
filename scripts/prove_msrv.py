#!/usr/bin/env python3
"""Prove the declared minimum supported Rust version by checking it in CI.

The declaration lives in `[workspace.package] rust-version`, so this script reads
it from the manifest instead of accepting a second, drift-prone copy. It then
checks every feature boundary with that toolchain, at every target, against the
committed lockfile.

The boundaries themselves are `check_features`'s matrix, not a second list: the
configurations CI checks on stable and the ones this proves at the floor are the
same rows, so the two jobs cannot disagree about what is supported.

A dependency that starts requiring a newer compiler makes `cargo` refuse to build
with "package X requires rustc Y", which fails this script: the advertised floor
cannot silently rot. Raising the floor is then a deliberate manifest change.
"""

from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

# This file is run both as `python scripts/prove_msrv.py` (Pixi, CI) and imported as
# `scripts.prove_msrv` (the pytest suite). Only the second has the repository root on
# `sys.path`, so put it there before reaching for the sibling module.
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.check_features import (
    CRATE_MANIFEST,
    WORKSPACE_MANIFEST,
    Failure,
    build_plan,
    combinations,
    load_manifest,
    package_name,
    run_plan,
)


def declared_msrv(manifest: dict, source: Path = WORKSPACE_MANIFEST) -> str:
    """Return `[workspace.package] rust-version`, the single source of the claim."""
    version = manifest.get("workspace", {}).get("package", {}).get("rust-version")
    if not isinstance(version, str) or not version.strip():
        raise ValueError(f"{source} declares no [workspace.package] rust-version")
    return version.strip()


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
        plan = build_plan(toolchain, package_name(crate), combinations(crate))
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
