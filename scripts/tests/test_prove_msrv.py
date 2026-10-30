"""The MSRV proof: declared version discovery, plan construction and failure handling."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from scripts.prove_msrv import (
    Failure,
    Plan,
    build_plan,
    declared_msrv,
    failure_hint,
    feature_sets,
    main,
    package_name,
    run_plan,
)

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


def test_package_name_is_read_from_the_crate_manifest():
    assert package_name(CRATE) == "clifacet"


def test_feature_sets_cover_the_empty_set_and_each_feature_but_not_default():
    assert feature_sets(CRATE) == [[], ["interactive"], ["json"], ["tui"]]


def test_plan_checks_each_feature_plus_one_all_features_run():
    plan = build_plan("1.88", "clifacet", feature_sets(CRATE))

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
    assert plan[1].command()[-2:] == ["--features", "interactive"]
    assert plan[-1].command() == ["cargo", "+1.88", "check", "--locked", "--all-targets", "--all-features"]


def test_a_plan_without_a_toolchain_uses_the_ambient_cargo():
    assert build_plan("", "clifacet", [[]])[0].command()[0:2] == ["cargo", "check"]


def test_run_plan_stops_at_the_first_failure():
    plan = [Plan("1.88", ("check", "a")), Plan("1.88", ("check", "b")), Plan("1.88", ("check", "c"))]
    seen = []

    def runner(command):
        seen.append(command[-1])
        return completed(0 if command[-1] != "b" else 1)

    failures = run_plan(plan, runner)

    assert [failure.plan for failure in failures] == [plan[1]]
    assert seen == ["a", "b"], "the last step must not run after a failure"


def test_run_plan_reports_no_failures_when_every_step_passes():
    assert run_plan([Plan("1.88", ("check",))], lambda _: completed(0)) == []


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
    from scripts.prove_msrv import CRATE_MANIFEST, WORKSPACE_MANIFEST, load_manifest

    workspace, crate = load_manifest(WORKSPACE_MANIFEST), load_manifest(CRATE_MANIFEST)
    plan = build_plan(declared_msrv(workspace), package_name(crate), feature_sets(crate))

    assert len(plan) == len(feature_sets(crate)) + 1
    assert all("--locked" in step.arguments for step in plan)
    assert Path(WORKSPACE_MANIFEST).name == "Cargo.toml"
