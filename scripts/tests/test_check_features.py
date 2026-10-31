"""The supported feature matrix: what is checked, and that nothing is left out.

CLI-16 asks CI to cover the empty set, the default set, each frontend alone and all
of them together. That is a claim about a *list*, so the list is data: these tests
assert what the list contains, that it is derived from the manifest rather than
hand-maintained, and that a manifest change cannot silently escape it.
"""

from __future__ import annotations

import subprocess

import pytest

from scripts.check_features import (
    Combination,
    Failure,
    Plan,
    build_plan,
    combinations,
    declared_features,
    default_features,
    describe_failure,
    main,
    package_name,
    run_plan,
)

CRATE = {
    "package": {"name": "clifacet"},
    "features": {
        "default": [],
        "interactive": ["dep:inquire"],
        "tui": ["dep:ratatui", "dep:crossterm"],
        "json": ["dep:serde_json"],
    },
}

WITH_A_DEFAULT = {
    "package": {"name": "clifacet"},
    "features": {"default": ["interactive"], "interactive": ["dep:inquire"], "tui": ["dep:ratatui"]},
}


def completed(code: int) -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess(args=["cargo"], returncode=code)


def test_declared_features_exclude_the_default_set():
    assert declared_features(CRATE) == ["interactive", "json", "tui"]


def test_the_default_set_is_read_from_the_manifest():
    assert default_features(CRATE) == []
    assert default_features(WITH_A_DEFAULT) == ["interactive"]


def test_the_matrix_is_the_documented_five():
    """No-free, default, each frontend, and all of them — the CLI-16 list."""
    assert [combination.name for combination in combinations(CRATE)] == [
        "no-default-features",
        "default",
        "interactive",
        "json",
        "tui",
        "all-features",
    ]


def test_an_empty_default_set_collapses_onto_the_empty_one():
    """`default` is empty here, so the two rows are one command — and the plan says so."""
    plan = build_plan("", "clifacet", combinations(CRATE))

    empty = [step for step in plan if step.arguments[-1] == "--no-default-features"]
    assert len(empty) == 1, "the same command must not be run twice"
    assert empty[0].covers == ("no-default-features", "default")


def test_a_real_default_set_stops_coinciding_with_the_empty_one():
    """If `default` ever grows, it becomes a configuration of its own."""
    plan = build_plan("", "clifacet", combinations(WITH_A_DEFAULT))
    default = [step for step in plan if "default" in step.covers]

    assert len(default) == 1
    assert default[0].covers == ("default", "interactive"), (
        "the default set and the feature it names are one configuration"
    )
    assert default[0].arguments[-2:] == ("--features", "interactive")


def test_every_row_is_checked_exactly_once_and_no_row_is_invented():
    plan = build_plan("", "clifacet", combinations(CRATE))
    covered = [name for step in plan for name in step.covers]

    assert sorted(covered) == sorted(combination.name for combination in combinations(CRATE))


def test_the_plan_always_ends_with_all_features():
    plan = build_plan("", "clifacet", combinations(CRATE))

    assert plan[-1].arguments == ("check", "--locked", "--all-targets", "--all-features"), (
        "the combined boundary is checked workspace-wide, with no package scope"
    )
    assert plan[-1].covers == ("all-features",)


def test_a_toolchain_is_optional():
    assert build_plan("1.88", "clifacet", combinations(CRATE))[0].command()[:2] == ["cargo", "+1.88"]
    assert build_plan("", "clifacet", combinations(CRATE))[0].command()[:2] == ["cargo", "check"]


def test_each_step_checks_a_named_package_with_all_targets():
    plan = build_plan("", "clifacet", combinations(CRATE))
    singles = [step for step in plan if step.covers != ("all-features",)]

    for step in singles:
        assert "-p" in step.arguments
        assert "clifacet" in step.arguments
        assert "--all-targets" in step.arguments
        assert "--locked" in step.arguments


def test_a_row_cannot_be_both_all_features_and_a_named_subset():
    """`all-features` means every feature; carrying a subset would check neither."""
    with pytest.raises(ValueError, match="all-features"):
        Combination(name="broken", features=("tui",), all_features=True).arguments("clifacet")


def test_run_plan_stops_at_the_first_failure():
    plan = [
        Plan("", ("check", "a"), ("a",)),
        Plan("", ("check", "b"), ("b",)),
        Plan("", ("check", "c"), ("c",)),
    ]
    seen = []

    def runner(command):
        seen.append(command[-1])
        return completed(0 if command[-1] != "b" else 1)

    failures = run_plan(plan, runner)

    assert [failure.plan for failure in failures] == [plan[1]]
    assert seen == ["a", "b"], "the last step must not run after a failure"


def test_run_plan_reports_no_failures_when_every_step_passes():
    assert run_plan([Plan("", ("check",), ("default",))], lambda _: completed(0)) == []


def test_a_failure_names_the_configuration_that_broke():
    failure = Failure(Plan("", ("check",), ("tui",)), 101, "error: no field `x`")

    assert "tui" in describe_failure(failure)
    assert "no field `x`" in describe_failure(failure)


def test_main_prints_the_matrix_without_running_it(capsys):
    assert main(["--print-matrix"]) == 0
    out = capsys.readouterr().out

    for name in ("no-default-features", "default", "interactive", "tui", "json", "all-features"):
        assert name in out


def test_main_prints_the_plan_without_executing_it(capsys):
    assert main(["--print-plan"]) == 0
    lines = capsys.readouterr().out.splitlines()

    assert lines and all(line.startswith("cargo check --locked --all-targets") for line in lines)
    assert any("--all-features" in line for line in lines)
    assert any("--no-default-features" in line for line in lines)


def test_the_committed_manifest_produces_the_documented_matrix():
    """The real crate, not a fixture: every declared feature is in the plan."""
    from scripts.check_features import CRATE_MANIFEST, load_manifest

    crate = load_manifest(CRATE_MANIFEST)
    names = {combination.name for combination in combinations(crate)}

    assert names == {"no-default-features", "default", "all-features", *declared_features(crate)}
    assert package_name(crate) == "clifacet"
