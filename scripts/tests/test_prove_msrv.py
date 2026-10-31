"""The MSRV proof: declared version discovery, plan construction and failure handling."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from scripts.check_features import Failure, Plan, build_plan, combinations, run_plan
from scripts.prove_msrv import declared_msrv, failure_hint, main

WORKSPACE = {"workspace": {"package": {"version": "0.1.0", "rust-version": "1.88"}}}
CRATE = {
    "package": {"name": "clifacet"},
    "features": {"default": [], "interactive": ["dep:inquire"], "tui": ["dep:ratatui"], "json": ["dep:serde_json"]},
}


def completed(code):
    return subprocess.CompletedProcess(args=["cargo"], returncode=code)


def test_declared_msrv_comes_from_the_workspace_manifest():
    assert declared_msrv(WORKSPACE) == "1.88"


@pytest.mark.parametrize(
    "manifest",
    [
        pytest.param({}, id="no-workspace"),
        pytest.param({"workspace": {}}, id="no-package-table"),
        pytest.param({"workspace": {"package": {}}}, id="no-rust-version"),
        pytest.param({"workspace": {"package": {"rust-version": "  "}}}, id="blank-rust-version"),
    ],
)
def test_missing_declaration_is_an_error(manifest):
    with pytest.raises(ValueError, match="rust-version"):
        declared_msrv(manifest)


def test_the_msrv_matrix_is_the_supported_feature_matrix():
    """One list, two jobs: what stable checks is what the floor proves."""
    plan = build_plan("1.88", "clifacet", combinations(CRATE))

    assert plan[0].command() == [
        "cargo",
        "+1.88",
        "check",
        "--locked",
        "--all-targets",
        "-p",
        "clifacet",
        "--no-default-features",
    ]
    assert plan[-1].command() == ["cargo", "+1.88", "check", "--locked", "--all-targets", "--all-features"]


def test_a_missing_toolchain_is_reported_as_a_rustup_problem():
    failure = Failure(Plan("1.88", ("check",)), 1, "error: no such command: `+1.88`")
    hint = failure_hint(failure)

    assert "rustup" in hint
    assert "rustup toolchain install 1.88" in hint


def test_a_compiler_too_old_is_reported_as_a_dependency_problem():
    failure = Failure(Plan("1.88", ("check",)), 101, "error: package `x` requires rustc 1.90")
    hint = failure_hint(failure)

    assert "rust-version" in hint
    assert "rustup toolchain" not in hint


def test_an_explicit_empty_toolchain_uses_the_ambient_cargo(capsys):
    assert main(["--toolchain", "", "--print-plan"]) == 0
    assert capsys.readouterr().out.splitlines()[0].startswith("cargo check ")


def test_main_prints_the_declared_toolchain_for_ci(capsys):
    assert main(["--print-toolchain"]) == 0
    assert capsys.readouterr().out.strip() == "1.88"


def test_main_prints_the_plan_without_executing_it(capsys):
    assert main(["--print-plan"]) == 0
    out = capsys.readouterr().out.splitlines()
    assert out and all(line.startswith("cargo +1.88 check --locked --all-targets") for line in out)
    assert any("--all-features" in line for line in out)


def test_the_committed_manifests_produce_a_usable_plan():
    from scripts.check_features import CRATE_MANIFEST, WORKSPACE_MANIFEST, load_manifest, package_name

    workspace, crate = load_manifest(WORKSPACE_MANIFEST), load_manifest(CRATE_MANIFEST)
    plan = build_plan(declared_msrv(workspace), package_name(crate), combinations(crate))

    assert len(plan) == len(combinations(crate)) - 1, "the empty set and `default` collapse"
    assert all("--locked" in step.arguments for step in plan)
    assert Path(WORKSPACE_MANIFEST).name == "Cargo.toml"
