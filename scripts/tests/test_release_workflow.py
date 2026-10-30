"""The release workflow's ordering contract, checked without running it.

Publishing is the one step that cannot be undone, so the workflow has to prove
everything else first. The two properties here are the ones that have already
been wrong once: the changelog notes must be readable *before* the first publish,
and the tag step must use the notes that were verified rather than deriving its
own.
"""

from __future__ import annotations

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
