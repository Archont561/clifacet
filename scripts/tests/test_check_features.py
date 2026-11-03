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
    cfg_sites,
    combinations,
    declared_features,
    default_features,
    describe_failure,
    expect_unused_features,
    main,
    package_name,
    run_plan,
    unused_features,
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


# --- a declared feature has to reach the code it claims to describe -----------------

FEATURE_SITES = """
fn nothing() {}
#[cfg(feature = "tui")]
mod tui {}
#[cfg(all(feature = "interactive", feature = "tui"))]
fn both() {}
"""

# `feature = "tui"` inside a doc comment is prose about the code, not a site in it.
DOCUMENTED = """
//! Enable the `tui` feature to draw a form.
// # ![cfg(feature = "tui")]
fn prose() {}
"""


def test_cfg_sites_are_read_from_the_source():
    assert cfg_sites(FEATURE_SITES) == {"tui", "interactive"}


def test_a_documented_cfg_is_not_a_site():
    """A guide that says `cfg(feature = "tui")` has not gated anything."""
    assert cfg_sites(DOCUMENTED) == set()


def test_a_feature_with_a_cfg_site_is_not_reported():
    assert unused_features(CRATE, {"tui"}) == ["interactive", "json"]


def test_a_feature_with_no_cfg_site_is_reported():
    """The claim is `cfg(feature = "json")` somewhere, not a dependency spec."""
    assert unused_features(CRATE, {"interactive", "tui"}) == ["json"]


def test_the_report_names_every_offending_feature_in_order():
    assert unused_features(CRATE, set()) == ["interactive", "json", "tui"]


def test_an_exception_needs_a_written_reason():
    assert expect_unused_features({"json": "no JSON output is implemented yet"}) == ["json"]


def test_an_exception_without_a_reason_is_a_mistake():
    """Silence is the thing the guard exists to prevent, so it cannot be allowlisted."""
    with pytest.raises(ValueError, match="json"):
        expect_unused_features({"json": "   "})


def test_main_fails_when_a_declared_feature_has_no_cfg_site(tmp_path, capsys):
    """The guard, through the command's own seam, on a fixture crate."""
    crate = tmp_path / "crates" / "clifacet"
    (crate / "src").mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "clifacet"\n\n[features]\ndefault = []\njson = ["dep:serde_json"]\n',
        encoding="utf-8",
    )
    (crate / "src" / "lib.rs").write_text("pub fn nothing() {}\n", encoding="utf-8")

    assert main(["--check-cfg-sites"], crate_manifest=crate / "Cargo.toml") == 1
    captured = capsys.readouterr()
    assert "json" in captured.err
    assert "cfg(feature" in captured.err


def test_main_passes_when_every_feature_has_a_site(tmp_path):
    crate = tmp_path / "crates" / "clifacet"
    (crate / "src").mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "clifacet"\n\n[features]\ndefault = []\njson = ["dep:serde_json"]\n',
        encoding="utf-8",
    )
    (crate / "src" / "lib.rs").write_text(
        '#[cfg(feature = "json")]\npub fn failures() {}\n', encoding="utf-8"
    )

    assert main(["--check-cfg-sites"], crate_manifest=crate / "Cargo.toml") == 0


def test_the_declared_exceptions_still_describe_the_crate():
    """An exception that is no longer needed is a stale note, so it has to be removed."""
    from scripts.check_features import CRATE_MANIFEST, UNUSED_FEATURES, load_manifest

    crate = load_manifest(CRATE_MANIFEST)
    sites = cfg_sites(
        "\n".join(
            path.read_text(encoding="utf-8")
            for path in sorted((CRATE_MANIFEST.parent / "src").rglob("*.rs"))
        )
    )

    assert sorted(unused_features(crate, sites)) == sorted(UNUSED_FEATURES), (
        "the documented exceptions and the features the crate really leaves ungated "
        "are the same set: an exception that outlives its feature is stale, and an "
        "ungated feature without one is the failure this guard exists for"
    )
