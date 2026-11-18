"""The supported-platform and feature-matrix contract, checked without running CI.

Two claims are made about the workflow here, and neither can be proven by running one
job: that every supported operating system has a job, and that the set of feature
combinations is owned by a tested script rather than by hand-written `cargo` lines that
nobody re-reads when a feature is added.
"""

from __future__ import annotations

from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"

SUPPORTED_PLATFORMS = ("ubuntu-latest", "macos-latest", "windows-latest")


@pytest.fixture(scope="module")
def workflow() -> str:
    return WORKFLOW.read_text(encoding="utf-8")


def job(workflow: str, name: str) -> str:
    """The text of one job, from its key to the next key at the same indentation."""
    marker = f"\n  {name}:"
    start = workflow.index(marker) + len(marker)
    body = []
    for line in workflow[start:].splitlines(keepends=True):
        # A job key is indented exactly two spaces; anything deeper belongs to this job.
        if line.startswith("  ") and not line.startswith("   ") and line.rstrip().endswith(":"):
            break
        body.append(line)
    return "".join(body)


def test_every_supported_platform_has_a_job(workflow):
    matrix = job(workflow, "platforms")

    for platform in SUPPORTED_PLATFORMS:
        assert platform in matrix, f"{platform} is a supported platform and needs a job"


def test_the_platform_job_does_not_stop_at_the_first_failing_operating_system(workflow):
    matrix = job(workflow, "platforms")

    assert "fail-fast: false" in matrix, (
        "one broken platform must not hide whether the others pass"
    )


def test_the_platform_job_runs_the_headless_test_suite(workflow):
    matrix = job(workflow, "platforms")

    assert "cargo test --workspace --all-features --locked" in matrix
    assert "--doc" in matrix, "documentation examples are part of the suite too"


def test_the_platform_job_proves_the_empty_feature_set_builds(workflow):
    matrix = job(workflow, "platforms")

    assert "--no-default-features" in matrix, (
        "a dependency used only by one feature must not break the base build on any platform"
    )


def test_the_feature_matrix_is_owned_by_the_script(workflow):
    """No inline `--features a,b` checks: adding a feature must not require editing CI."""
    assert "pixi run --frozen check-features" in workflow
    assert "--features interactive" not in workflow
    assert "--features tui" not in workflow


def test_the_msrv_job_uses_the_same_matrix_script(workflow):
    msrv = job(workflow, "msrv")

    assert "prove_msrv.py" in msrv
    assert "--features" not in msrv, (
        "the MSRV job must not carry its own feature list; the script owns it"
    )
