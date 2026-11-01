"""The release workflow's ordering contract, checked without running it.

Publishing is the one step that cannot be undone, so the workflow has to prove
everything else first. The two properties here are the ones that have already
been wrong once: the changelog notes must be readable *before* the first publish,
and the tag step must use the notes that were verified rather than deriving its
own.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"


@pytest.fixture(scope="module")
def workflow() -> str:
    return WORKFLOW.read_text(encoding="utf-8")


def test_notes_are_resolved_before_the_first_publish(workflow):
    resolve = workflow.index("Resolve the release notes")
    publish = workflow.index("cargo publish")

    assert resolve < publish, (
        "a missing changelog section must fail the run before anything is published"
    )


def test_resolution_is_not_limited_to_a_real_release(workflow):
    """A dry run has to surface a missing section too, or the first sign is a half-publish."""
    step = workflow[workflow.index("Resolve the release notes"):]
    step = step[: step.index("- name:")]

    assert "if:" not in step, "the notes are resolved on every dispatch, dry run included"


def test_the_tag_step_reuses_the_resolved_notes(workflow):
    tag = workflow[workflow.index("Tag the release and attach the changelog notes"):]

    assert "notes_path" in tag, "the tag step should reuse the verified file"
    assert "bump_version.py --notes" not in tag, (
        "re-deriving the notes after publishing can fail on a release that already shipped"
    )


# The rest of the publication contract (CLI-18's AC#1): what the workflow is allowed to
# be, checked rather than trusted. Each of these has a failure mode that a rehearsal
# would only discover by publishing something.


def test_a_push_never_releases(workflow):
    triggers = workflow[workflow.index("on:"): workflow.index("permissions:")]

    assert "workflow_dispatch" in triggers
    assert "push" not in triggers, "trusted publishing is bound to a manual dispatch"


def test_the_job_runs_in_the_crates_environment(workflow):
    job = workflow[workflow.index("  publish:"):]

    assert "environment: crates" in job, "the environment is the authorization gate"


def test_oidc_is_scoped_to_the_publish_job(workflow):
    top = workflow[workflow.index("permissions:"): workflow.index("env:")]
    job = workflow[workflow.index("  publish:"):]

    assert "id-token: write" not in top, "a workflow-wide token scope is wider than publishing needs"
    assert "id-token: write" in job


def test_the_macro_crate_is_published_before_the_main_one(workflow):
    loop = workflow[workflow.index("Publish both crates in dependency order"):]

    assert loop.index('for crate in "$MACROS" "$MAIN"') < loop.index("cargo publish")


def test_the_token_exchange_only_happens_for_a_real_release(workflow):
    exchange = workflow[workflow.index("Exchange the OIDC token"):]

    assert "if: ${{ !inputs.dry_run }}" in exchange[:400], (
        "a dry run must not be able to obtain a publish token"
    )


def test_a_dry_run_is_the_default(workflow):
    inputs = workflow[workflow.index("workflow_dispatch:"): workflow.index("permissions:")]

    assert "default: true" in inputs, "a dispatch without a flag must not publish"


def test_every_action_is_pinned_to_a_commit(workflow):
    used = re.findall(r"uses: (\S+)", workflow)

    assert used, "the workflow should use actions at all"
    for action in used:
        assert re.fullmatch(r"[\w.-]+/[\w.-]+(?:/[\w.-]+)?@[0-9a-f]{40}", action), (
            f"{action} is not pinned to a commit SHA"
        )
