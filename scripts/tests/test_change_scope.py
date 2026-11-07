"""Which lane a change takes through CI, decided in Python and tested here.

A change that touches only documentation runs the checks documentation can break; any
other change runs all of CI. The decision has exactly one safe direction: when in doubt,
run everything. So an unrecognised path, an empty change, a dispatch, a branch's first
push and a diff git cannot compute all take the full lane, and the workflow is wired so
that a classifier that crashes takes it too.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from scripts.change_scope import DOCS_ONLY, diff_command, documentation_only, is_documentation, main

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"

BASE = "a" * 40
PUSHED_FROM = "b" * 40
FIRST_PUSH = "0" * 40

DOCUMENTATION = [
    "backlog/tasks/cli-40 - CI-a-documentation-lane-for-docs-only-changes.md",
    "backlog/config.yml",
    "backlog/decisions/decision-18 - Proposal.md",
    "docs/command-core.md",
    "README.md",
    "examples/simple/README.md",
    "AGENTS.md",
    "CHANGELOG.md",
    ".agents/skills/tdd/SKILL.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
]

CODE = [
    "Cargo.toml",
    "Cargo.lock",
    "crates/clifacet/src/lib.rs",
    # A guide's snippet is compiled from here, so regenerating it is a code change.
    "examples/simple/tests/documented_snippets.rs",
    "examples/backlog-lite/features/plan.feature",
    ".github/workflows/ci.yml",
    "pixi.toml",
    "pixi.lock",
    "scripts/check_docs.py",
    "lefthook.yml",
    ".alint.yml",
    "codecov.yml",
    "docsite/index.md.rs",
    "crates/clifacet/LICENSE-MIT",
]


class Git:
    """A stand-in for `git`: records each call and answers with a canned result."""

    def __init__(self, code: int = 0, paths: list[str] | None = None):
        self.code = code
        self.output = "".join(f"{path}\0" for path in paths or [])
        self.calls: list[list[str]] = []

    def __call__(self, argv):
        self.calls.append(list(argv))
        return self.code, self.output


@pytest.mark.parametrize("path", DOCUMENTATION)
def test_documentation_paths_are_documentation(path):
    assert is_documentation(path)


@pytest.mark.parametrize("path", CODE)
def test_every_other_path_is_code(path):
    assert not is_documentation(path)


def test_a_change_of_only_documentation_takes_the_documentation_lane():
    assert documentation_only(DOCUMENTATION)


def test_one_code_path_puts_a_change_in_the_full_lane():
    assert not documentation_only([*DOCUMENTATION, "crates/clifacet/src/lib.rs"])


def test_an_empty_change_takes_the_full_lane():
    """Nothing to classify is not evidence that nothing needs checking."""
    assert not documentation_only([])


def test_the_diff_reports_a_rename_as_a_deletion_and_an_addition():
    """With rename detection, moving `src/x.rs` to `docs/x.md` would list only the `.md`."""
    command = diff_command(f"{BASE}..HEAD")

    assert "--no-renames" in command
    assert "-z" in command, "a backlog file name has spaces in it"
    assert command[-1] == f"{BASE}..HEAD"


def run(tmp_path, env, git):
    output = tmp_path / "output"
    summary = tmp_path / "summary"
    code = main(
        [],
        env={**env, "GITHUB_OUTPUT": str(output), "GITHUB_STEP_SUMMARY": str(summary)},
        git=git,
    )
    return code, output.read_text(encoding="utf-8"), summary.read_text(encoding="utf-8")


def test_a_documentation_pull_request_takes_the_documentation_lane(tmp_path):
    git = Git(paths=["docs/command-core.md", "backlog/tasks/cli-40 - CI.md"])

    code, output, summary = run(tmp_path, {"EVENT": "pull_request", "PR_BASE": BASE}, git)

    assert code == 0
    assert output == f"{DOCS_ONLY}=true\n"
    assert git.calls == [diff_command(f"{BASE}..HEAD")]
    assert "docs/command-core.md" in summary


def test_a_pull_request_with_code_takes_the_full_lane(tmp_path):
    git = Git(paths=["README.md", "crates/clifacet/src/lib.rs"])

    code, output, summary = run(tmp_path, {"EVENT": "pull_request", "PR_BASE": BASE}, git)

    assert code == 0
    assert output == f"{DOCS_ONLY}=false\n"
    assert "crates/clifacet/src/lib.rs" in summary


def test_a_push_compares_against_the_commit_it_pushed_from(tmp_path):
    git = Git(paths=["CHANGELOG.md"])

    _, output, _ = run(tmp_path, {"EVENT": "push", "PUSH_BASE": PUSHED_FROM}, git)

    assert output == f"{DOCS_ONLY}=true\n"
    assert git.calls == [diff_command(f"{PUSHED_FROM}..HEAD")]


@pytest.mark.parametrize(
    "env",
    [
        {"EVENT": "push", "PUSH_BASE": FIRST_PUSH},
        {"EVENT": "workflow_dispatch"},
        {},
    ],
    ids=["first push", "dispatch", "no event"],
)
def test_a_run_with_nothing_to_compare_against_takes_the_full_lane(tmp_path, env):
    git = Git(paths=["README.md"])

    code, output, _ = run(tmp_path, env, git)

    assert code == 0
    assert output == f"{DOCS_ONLY}=false\n"
    assert git.calls == [], "there is no range to diff"


def test_a_diff_git_cannot_compute_takes_the_full_lane(tmp_path):
    """A force-pushed branch's before-SHA is not in the clone, and `git diff` says so."""
    git = Git(code=128, paths=["README.md"])

    code, output, summary = run(tmp_path, {"EVENT": "push", "PUSH_BASE": PUSHED_FROM}, git)

    assert code == 0
    assert output == f"{DOCS_ONLY}=false\n"
    assert "git" in summary


def test_a_missing_git_takes_the_full_lane(tmp_path):
    def no_git(_argv):
        raise FileNotFoundError("git")

    code, output, _ = run(tmp_path, {"EVENT": "pull_request", "PR_BASE": BASE}, no_git)

    assert code == 0
    assert output == f"{DOCS_ONLY}=false\n"


def test_no_crate_or_example_compiles_a_markdown_file_in():
    """The classifier calls every `.md` file documentation. That is only true while no
    Rust source reads one in; a `#![doc = include_str!("../README.md")]` would make a
    README edit a change to the crate's doctests, and this test is the reminder."""
    including = re.compile(r"include_(?:str|bytes)!\s*\(\s*\"[^\"]*\.md\"")
    offenders = [
        str(path.relative_to(ROOT))
        for top in ("crates", "examples")
        for path in (ROOT / top).rglob("*.rs")
        if "target" not in path.parts and including.search(path.read_text(encoding="utf-8"))
    ]

    assert offenders == [], (
        "these files compile Markdown in, so scripts/change_scope.py must stop treating "
        f"every .md file as documentation: {offenders}"
    )


# The wiring, read from the workflow text: the lane is only as safe as these conditions.

FULL_JOBS = ("quality", "coverage", "msrv", "platforms")


@pytest.fixture(scope="module")
def workflow() -> str:
    return WORKFLOW.read_text(encoding="utf-8")


def job(workflow: str, name: str) -> str:
    """The text of one job, from its key to the next key at the same indentation."""
    marker = f"\n  {name}:\n"
    start = workflow.index(marker) + len(marker)
    body = []
    for line in workflow[start:].splitlines(keepends=True):
        if line.startswith("  ") and not line.startswith("   ") and line.rstrip().endswith(":"):
            break
        body.append(line)
    return "".join(body)


def condition(block: str) -> str:
    found = re.search(r"^    if: (.+)$", block, re.MULTILINE)
    return found.group(1).strip() if found else ""


def test_the_classifier_runs_the_tested_script(workflow):
    classifier = job(workflow, "changes")

    assert "python3 scripts/change_scope.py" in classifier
    assert "fetch-depth: 0" in classifier, "the diff needs the base commit in the clone"
    assert f"{DOCS_ONLY}: ${{{{ steps.scope.outputs.{DOCS_ONLY} }}}}" in classifier
    assert condition(classifier) == "", "the classifier itself always runs"


@pytest.mark.parametrize("name", FULL_JOBS)
def test_every_full_job_is_skipped_only_on_a_documentation_change(workflow, name):
    block = job(workflow, name)

    assert "needs: changes" in block
    assert condition(block) == f"${{{{ !cancelled() && needs.changes.outputs.{DOCS_ONLY} != 'true' }}}}", (
        "!cancelled() keeps the job running when the classifier fails, and != 'true' "
        "treats an empty output as code"
    )


def test_the_documentation_job_runs_only_on_a_documentation_change(workflow):
    block = job(workflow, "documentation")

    assert "needs: changes" in block
    assert condition(block) == f"${{{{ needs.changes.outputs.{DOCS_ONLY} == 'true' }}}}"
    assert "fetch-depth: 0" in block, "the changelog check renders from the history"


@pytest.mark.parametrize(
    "task",
    ["alint", "skills-check", "backlog-check", "check-docs", "changelog-check"],
)
def test_the_documentation_job_runs_each_check_documentation_can_break(workflow, task):
    assert f"pixi run --frozen {task}" in job(workflow, "documentation")
    assert f"pixi run --frozen {task}" in job(workflow, "quality"), (
        "the documentation lane is a subset of the full lane, never a check of its own"
    )


def test_commit_messages_are_checked_on_every_change(workflow):
    block = job(workflow, "commit-messages")

    assert "needs:" not in block
    assert condition(block) == ""
