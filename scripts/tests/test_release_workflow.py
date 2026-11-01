"""The two authored workflows as contracts: plumbing here, decisions in Python.

`release.yml` used to carry the release logic — a notes dance, a version-consistency
`for` loop over `sed`, a publish loop with index polling, and a summary built by hand.
None of it could be exercised without dispatching a run, and the questions it answered
were asked at most once. That logic now lives in `scripts/release.py`, tested by
`test_release.py`; this file checks the other direction, that the workflow really is only
plumbing, and that the properties a dispatch would otherwise reveal are still pinned.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
RELEASE = WORKFLOWS / "release.yml"
CI = WORKFLOWS / "ci.yml"

# Shell control flow in a `run:` block is the smell this repository is removing: it is
# untestable without a runner, it fails for a reason the script would have named, and it
# duplicates a decision that belongs somewhere a test can reach.
SHELL_LOGIC = ("$( ", "$(", "&&", "||", "if ", "for ", "while ", "case ", "done", " | ", "> ")

AUTHORED = (CI, RELEASE)


@pytest.fixture(scope="module")
def workflow() -> str:
    return RELEASE.read_text(encoding="utf-8")


def run_blocks(workflow: str) -> list[str]:
    """Every `run:` value, including block scalars, without the YAML around it."""
    blocks = []
    lines = workflow.splitlines()
    for index, line in enumerate(lines):
        match = re.match(r"^(\s*)run:\s*(.*)$", line)
        if match is None:
            continue
        indent, inline = match.groups()
        if inline and inline not in {"|", ">", "|-", ">-"}:
            blocks.append(inline)
            continue
        body = []
        for following in lines[index + 1 :]:
            if following.strip() and len(following) - len(following.lstrip()) <= len(indent):
                break
            body.append(following)
        blocks.append("\n".join(body))
    return blocks


# --- the decisions are not in the workflow ----------------------------------


@pytest.mark.parametrize("path", AUTHORED, ids=lambda path: path.name)
def test_no_workflow_step_contains_shell_logic(path):
    offenders = [
        block.strip()
        for block in run_blocks(path.read_text(encoding="utf-8"))
        for token in SHELL_LOGIC
        if token in block
    ]

    assert offenders == [], (
        "move the decision into a script the tests can reach, and let the step call it"
    )


@pytest.mark.parametrize("path", AUTHORED, ids=lambda path: path.name)
def test_every_step_calls_a_pixi_task_or_a_tool_directly(path):
    """`pixi run <task>` here, `cargo`/`rustup` only where Pixi is deliberately absent."""
    for block in run_blocks(path.read_text(encoding="utf-8")):
        for line in block.splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            assert stripped.split()[0] in {
                "pixi", "cargo", "rustc", "rustup", "python3",
            }, f"{path.name} still runs `{stripped}`"


def test_the_release_workflow_delegates_every_step_to_the_release_script(workflow):
    tasks = re.findall(r"pixi run --frozen release (\w+)", workflow)

    assert tasks == ["preflight", "package", "report", "publish", "tag", "changelog"], (
        "the six decisions live in scripts/release.py; the workflow only calls them"
    )


def test_the_release_workflow_carries_no_crate_names_or_index_urls(workflow):
    """Those are constants in the script; a copy here is a second source of truth."""
    for literal in ("clifacet-macros", "index.crates.io", "cl/if", "cargo publish", "curl"):
        assert literal not in workflow, f"{literal!r} belongs in scripts/release.py"


# --- the plumbing that is genuinely the workflow's job ----------------------


def test_a_push_never_releases(workflow):
    triggers = workflow[workflow.index("on:"): workflow.index("permissions:")]

    assert "workflow_dispatch" in triggers
    assert "push" not in triggers, "trusted publishing is bound to a manual dispatch"


def test_the_job_runs_in_the_crates_environment(workflow):
    assert "environment: crates" in workflow[workflow.index("  publish:"):]


def test_oidc_is_scoped_to_the_publish_job(workflow):
    top = workflow[workflow.index("permissions:"): workflow.index("jobs:")]
    job = workflow[workflow.index("  publish:"):]

    assert "id-token: write" not in top, "a workflow-wide token scope is wider than publishing needs"
    assert "id-token: write" in job


def test_the_token_exchange_only_happens_for_a_real_release(workflow):
    exchange = workflow[workflow.index("Exchange the OIDC token"):]

    assert "if: ${{ !inputs.dry_run }}" in exchange[:400], (
        "a dry run must not be able to obtain a publish token"
    )


def test_a_dry_run_is_the_default(workflow):
    inputs = workflow[workflow.index("workflow_dispatch:"): workflow.index("permissions:")]

    assert "default: true" in inputs, "a dispatch without a flag must not publish"


def test_a_dry_run_reports_instead_of_publishing(workflow):
    report = workflow[workflow.index("Report the dry run"):]
    publish = workflow[workflow.index("Publish both crates"):]

    assert "if: inputs.dry_run" in report[:400]
    assert "if: ${{ !inputs.dry_run }}" in publish[:400]


def test_the_tag_step_uses_the_notes_preflight_verified(workflow):
    tag = workflow[workflow.index("Tag the release"):]

    assert "steps.preflight.outputs.notes_path" in tag, (
        "re-deriving the notes after publishing can fail on a release that already shipped"
    )


def test_the_changelog_is_regenerated_only_after_the_tag_exists(workflow):
    tag = workflow.index("Tag the release")
    regen = workflow.index("Regenerate the changelog")
    step = workflow[regen:]

    assert tag < regen, "a version section names a tag, and the tag is created above"
    assert "if: ${{ !inputs.dry_run }}" in step[:400], "a dry run publishes nothing, so it tags nothing"
    assert "GITHUB_TOKEN" in step[:600], "the push needs a token the checkout does not persist"


def test_the_changelog_check_runs_in_ci_with_the_history_it_needs():
    ci = CI.read_text(encoding="utf-8")
    quality = ci[ci.index("  quality:"):]

    assert "pixi run --frozen changelog-check" in quality
    assert "fetch-depth: 0" in quality[:3000], "convco reads tags and commits, not a shallow clone"


def test_every_action_is_pinned_to_a_commit(workflow):
    used = re.findall(r"uses: (\S+)", workflow)

    assert used, "the workflow should use actions at all"
    for action in used:
        assert re.fullmatch(r"[\w.-]+/[\w.-]+(?:/[\w.-]+)?@[0-9a-f]{40}", action), (
            f"{action} is not pinned to a commit SHA"
        )


# --- the CI job's decisions are in scripts too ------------------------------


def test_the_commit_message_job_runs_the_script_that_resolves_its_range():
    ci = CI.read_text(encoding="utf-8")
    job = ci[ci.index("  commit-messages:"):]

    assert "pixi run --frozen commit-range check" in job
    assert "GITHUB_OUTPUT" not in job, "the range is decided inside the script now"
    assert job.count("convco") <= 1, "the script invokes convco; the workflow names it once in prose"


def test_the_advisory_cache_key_and_cleanup_are_scripts():
    ci = CI.read_text(encoding="utf-8")

    assert "pixi run --frozen advisory-cache key" in ci
    assert "pixi run --frozen advisory-cache clean" in ci
    assert "git -C" not in ci, "the reset belongs to scripts/advisory_cache.py"


def test_the_msrv_job_asks_the_script_for_its_output():
    ci = CI.read_text(encoding="utf-8")

    assert "python3 scripts/prove_msrv.py --github-output version" in ci
    assert "GITHUB_OUTPUT" not in ci
