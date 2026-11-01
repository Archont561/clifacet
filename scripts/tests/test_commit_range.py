"""The range the commit-message check covers, decided in Python instead of in YAML.

The `commit-messages` job used to work this out inline: an `if`/`elif`/`else` writing a
step output, then a second step branching on whether that output was empty, because the
runner's actionlint rejects the unquoted expansion the one-line form would need. None of
that could be tested without pushing a branch and watching a job.

The decision itself is small and well defined:

- a pull request compares its base commit with the head, whatever the branch;
- a push compares the pushed-from commit, except on a branch's first push, where the
  before-SHA is all zeros and there is no base to compare against;
- with no range at all, the tip plus a small window is checked, which still catches the
  message written by the push that created the branch.
"""

from __future__ import annotations

import pytest

from scripts.commit_range import DEFAULT_DEPTH, command, main, resolve_range

FIRST_PUSH = "0" * 40
BASE = "a" * 40
HEAD_BEFORE = "b" * 40


def test_a_pull_request_compares_against_its_base_commit():
    assert resolve_range("pull_request", pr_base=BASE) == f"{BASE}..HEAD"


def test_a_push_compares_against_the_commit_it_pushed_from():
    assert resolve_range("push", push_base=HEAD_BEFORE) == f"{HEAD_BEFORE}..HEAD"


def test_a_first_push_has_no_base_so_the_range_is_empty():
    """All zeros is git's way of saying 'this branch did not exist before'."""
    assert resolve_range("push", push_base=FIRST_PUSH) == ""


def test_a_dispatch_has_no_base_either():
    assert resolve_range("workflow_dispatch") == ""


def test_a_pull_request_wins_over_a_stale_push_base():
    assert resolve_range("pull_request", pr_base=BASE, push_base=HEAD_BEFORE) == f"{BASE}..HEAD"


def test_a_range_becomes_one_convco_invocation():
    assert command("abc..HEAD") == ["convco", "check", "abc..HEAD"]


def test_no_range_becomes_the_tip_plus_a_window():
    """Two invocations, not one expanded variable: the unquoted form trips shellcheck."""
    assert command("") == ["convco", "check", "-n", str(DEFAULT_DEPTH)]


def test_main_runs_convco_for_the_event_it_was_given():
    seen = []

    def run(argv):
        seen.append(argv)
        return 0

    assert main([], env={"EVENT": "pull_request", "PR_BASE": BASE}, run=run) == 0
    assert seen == [["convco", "check", f"{BASE}..HEAD"]]


def test_main_propagates_a_failed_check():
    def run(argv):
        return 1

    assert main([], env={"EVENT": "pull_request", "PR_BASE": BASE}, run=run) == 1


def test_main_falls_back_to_the_window_when_there_is_no_base():
    seen = []

    def run(argv):
        seen.append(argv)
        return 0

    assert main([], env={"EVENT": "workflow_dispatch"}, run=run) == 0
    assert seen == [["convco", "check", "-n", str(DEFAULT_DEPTH)]]


def test_main_explains_a_missing_convco(capsys):
    def run(argv):
        raise FileNotFoundError("convco")

    assert main([], env={"EVENT": "push", "PUSH_BASE": HEAD_BEFORE}, run=run) == 1
    assert "convco" in capsys.readouterr().err


@pytest.mark.parametrize(
    ("event", "expected"),
    [("pull_request", "..HEAD"), ("push", "..HEAD"), ("workflow_dispatch", "")],
)
def test_every_event_resolves_to_something_convco_accepts(event, expected):
    """The result is either a git range or the empty string the caller turns into `-n`."""
    resolved = resolve_range(event, pr_base="c" * 40, push_base="d" * 40)

    assert resolved.endswith(expected)
    assert " " not in resolved
