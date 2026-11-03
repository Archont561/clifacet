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

A row that compiles proves the feature *can* be enabled, not that enabling it does
anything. So the same pass also checks the other half of the claim: every declared
feature has a `cfg(feature = "…")` site in the crate's source. A feature with no
site is a promise the crate does not keep, and the matrix row it owns is theatre.
The unbuilt ones are named in `UNUSED_FEATURES` with the reason and the card that
will settle them; silencing a feature there is the only way past the guard, and it
costs a sentence.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from collections.abc import Mapping, Sequence
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

# A declared feature that no `cfg` mentions is a promise the crate does not keep: it
# compiles in the matrix and changes nothing. The few that are genuinely unbuilt yet are
# named here with the reason, and nothing else may be: a silently allowlisted feature is
# exactly the rot this guard exists to catch.
#
# CLI-36 decides `json`'s fate — either it gets a `cfg(feature = "json")` site that
# serialises failures, or it is retired from the manifest. When that lands, delete the
# entry; `test_the_declared_exceptions_still_describe_the_crate` fails if it outlives it.
UNUSED_FEATURES: dict[str, str] = {
    "json": (
        "reserves `serde_json` for machine-readable output that is not implemented yet, "
        "so no code is gated on it; CLI-36 either builds it on a real cfg site or retires "
        "the feature"
    ),
}

# `cfg(feature = "x")`, `cfg!(feature = "x")` and `cfg_attr(…, feature = "x")` are all
# sites where a feature changes the code. Comments are stripped first, because a guide
# that *documents* a gate is not one.
CFG_INVOCATION = re.compile(r"\bcfg(?:_attr)?!?\s*\(")
FEATURE_PREDICATE = re.compile(r'feature\s*=\s*"([^"]+)"')



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


def cfg_sites(source: str) -> set[str]:
    """Every feature named by a `cfg` in Rust source, ignoring comments.

    A bare `cfg(feature = "…")` token in a doc comment is prose about the code, not a
    gate on it, so comments are removed before scanning. The parens are matched by hand
    rather than with a regex because `cfg(all(unix, feature = "tui"))` nests.
    """
    code = strip_comments(source)
    sites: set[str] = set()
    for invocation in CFG_INVOCATION.finditer(code):
        start = invocation.end() - 1
        depth = 0
        for position in range(start, len(code)):
            if code[position] == "(":
                depth += 1
            elif code[position] == ")":
                depth -= 1
                if depth == 0:
                    sites.update(FEATURE_PREDICATE.findall(code[start : position + 1]))
                    break
    return sites


def strip_comments(source: str) -> str:
    """Remove `//` and `/* */` comments, leaving strings and code alone.

    Not a Rust parser: it tracks string and char literals only far enough that a `//`
    inside one is not mistaken for a comment. That is the narrow job this needs — the
    constructor to be gated is always in the crate's own source, which `cargo fmt` and
    clippy have already put in ordinary shapes.
    """
    result: list[str] = []
    index = 0
    while index < len(source):
        character = source[index]
        if character == '"':
            end = index + 1
            while end < len(source) and source[end] != '"':
                end += 2 if source[end] == "\\" else 1
            result.append(source[index : end + 1])
            index = end + 1
        elif source.startswith("//", index):
            end = source.find("\n", index)
            index = len(source) if end == -1 else end
        elif source.startswith("/*", index):
            end = source.find("*/", index + 2)
            index = len(source) if end == -1 else end + 2
        else:
            result.append(character)
            index += 1
    return "".join(result)


def unused_features(manifest: dict, sites: set[str]) -> list[str]:
    """Declared features that no cfg site mentions, in manifest order.

    Deliberately knows nothing about exceptions: this is the fact the guard acts on, and
    keeping the policy out of it is what lets a test compare the fact against the
    documented allowlist.
    """
    return [name for name in declared_features(manifest) if name not in sites]


def expect_unused_features(exceptions: Mapping[str, str]) -> list[str]:
    """The allowlist, refusing an entry that has no written reason."""
    for name, reason in exceptions.items():
        if not reason.strip():
            raise ValueError(
                f"feature `{name}` is allowlisted without a reason; an exception has to say "
                f"why the feature is declared but not gated, or the guard means nothing"
            )
    return sorted(exceptions)


def crate_sources(crate_manifest: Path = CRATE_MANIFEST) -> list[Path]:
    """The crate's own Rust files, which are the only place a site can be."""
    return sorted((crate_manifest.parent / "src").rglob("*.rs"))


def check_cfg_sites(crate_manifest: Path = CRATE_MANIFEST) -> list[str]:
    """Every declared feature with no cfg site and no written exception.

    `UNUSED_FEATURES` records promises about *this* repository's crate, so it is granted
    to that crate and no other: a copy of the source elsewhere has not earned the excuse.
    """
    crate = load_manifest(crate_manifest)
    sites = cfg_sites(
        "\n".join(path.read_text(encoding="utf-8") for path in crate_sources(crate_manifest))
    )
    allowed = expect_unused_features(UNUSED_FEATURES if crate_manifest == CRATE_MANIFEST else {})
    return [name for name in unused_features(crate, sites) if name not in allowed]


def describe_unused(names: Sequence[str], exceptions: Mapping[str, str]) -> str:
    """Name the ungated features the guard is failing on, and what to do about each."""
    lines = [
        "a declared feature has to gate something: no `cfg(feature = \"…\")` site mentions "
        "these, so enabling them changes nothing and their matrix row proves nothing."
    ]
    for name in names:
        lines.append(f"  {name}: add a `cfg(feature = \"{name}\")` site, or retire the feature")
    lines.append(
        f"an unbuilt feature may be allowlisted in UNUSED_FEATURES with a written reason; "
        f"currently allowlisted: {', '.join(sorted(exceptions)) or 'none'}"
    )
    return "\n".join(lines)


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


def main(argv: Sequence[str] | None = None, crate_manifest: Path = CRATE_MANIFEST) -> int:
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
    parser.add_argument(
        "--check-cfg-sites",
        action="store_true",
        help="only check that every declared feature gates some code, and exit",
    )
    arguments = parser.parse_args(argv)

    try:
        crate = load_manifest(crate_manifest)
        matrix = combinations(crate)
        plan = build_plan(arguments.toolchain, package_name(crate, crate_manifest), matrix)
        ungated = check_cfg_sites(crate_manifest)
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

    # Checked before anything is compiled: a feature that gates nothing is a manifest
    # mistake, and finding it after four `cargo check` runs wastes the slowest part.
    if ungated:
        print(f"check-features: {describe_unused(ungated, UNUSED_FEATURES)}", file=sys.stderr)
        return 1

    if arguments.check_cfg_sites:
        print(f"check-features: every declared feature gates some code ({len(declared_features(crate))} checked)")
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
