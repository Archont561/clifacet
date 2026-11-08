"""The backlog status guard: what it reports, and what it deliberately leaves alone.

CLI-49, CLI-50 and CLI-38 all drifted the same way — the work finished and the card did
not say so — and nothing noticed for hours. These tests pin the two rules that would have
caught them, and the two judgement calls the guard makes: only the criteria block counts,
and a card with no criteria is not this guard's business.
"""

from __future__ import annotations

import textwrap

from scripts.check_backlog_status import load, main, parse, problems

CARD = textwrap.dedent(
    """\
    ---
    id: CLI-99
    title: "A task"
    status: {status}
    ---

    ## Description

    Some prose.

    ## Acceptance Criteria
    <!-- AC:BEGIN -->
    {criteria}
    <!-- AC:END -->

    ## Implementation Notes
    """
)


def write(tmp_path, name: str, status: str, criteria: str):
    """One card on disk, with the frontmatter and criteria a task file really has."""
    path = tmp_path / f"{name}.md"
    path.write_text(CARD.format(status=status, criteria=criteria), encoding="utf-8")
    return path


def test_a_done_task_with_an_unchecked_criterion_is_reported(tmp_path):
    """The fault that makes a board lie about finished work: it claims more than it proved."""
    write(tmp_path, "cli-99 - A-task", "Done", "- [x] #1 met\n- [ ] #2 not met")

    found = problems(load(tmp_path))

    assert len(found) == 1
    assert "CLI-99" in found[0]
    assert "1 of 2" in found[0]


def test_a_finished_task_that_is_not_done_is_reported(tmp_path):
    """The fault CLI-49 and CLI-50 shared: every criterion met, status still open."""
    write(tmp_path, "cli-99 - A-task", "In Progress", "- [x] #1 met\n- [x] #2 met")

    found = problems(load(tmp_path))

    assert len(found) == 1
    assert "CLI-99" in found[0]
    assert "To Do" not in found[0] and "In Progress" in found[0]


def test_agreement_is_silent(tmp_path):
    """Both honest shapes pass: finished and Done, unfinished and open."""
    write(tmp_path, "cli-98 - Done-one", "Done", "- [x] #1 met")
    write(tmp_path, "cli-99 - Open-one", "To Do", "- [ ] #1 open")

    assert problems(load(tmp_path)) == []


def test_a_task_with_no_criteria_is_not_checked(tmp_path):
    """A card that is a note to self has nothing to be consistent with."""
    write(tmp_path, "cli-99 - A-note", "Done", "no criteria here")

    assert problems(load(tmp_path)) == []


def test_only_the_criteria_block_counts(tmp_path):
    """A tick in the notes is prose about the work, not a claim about done.

    Without this, a task could satisfy the guard by mentioning a checked box in its
    notes while its real criteria stayed open — the guard would be measuring the wrong
    text.
    """
    path = write(tmp_path, "cli-99 - A-task", "Done", "- [ ] #1 open")
    path.write_text(
        path.read_text(encoding="utf-8") + "\n- [x] this box is in the notes\n",
        encoding="utf-8",
    )

    assert problems(load(tmp_path)) == [
        "CLI-99 is Done with 1 of 1 acceptance criteria unchecked"
    ]


def test_the_identifier_is_the_board_id_not_the_filename(tmp_path):
    """A message has to name the card a maintainer would search the board for."""
    path = write(tmp_path, "cli-42 - Some-long-title", "Done", "- [ ] #1 open")

    assert parse(path).identifier == "CLI-42"


def test_a_file_without_frontmatter_is_ignored(tmp_path):
    """`backlog/tasks` is a folder of Markdown; a stray note in it is not a card."""
    (tmp_path / "notes.md").write_text("just a note\n", encoding="utf-8")

    assert load(tmp_path) == []


def test_the_check_fails_and_says_how_to_resolve_it(tmp_path, capsys):
    """The exit code is what CI reads, and the guidance is what a person reads."""
    write(tmp_path, "cli-99 - A-task", "Done", "- [ ] #1 open")

    assert main([], tmp_path) == 1
    captured = capsys.readouterr()
    assert "CLI-99" in captured.err
    assert "Fix the status" in captured.err


def test_the_check_passes_on_an_agreed_tree(tmp_path, capsys):
    write(tmp_path, "cli-99 - A-task", "Done", "- [x] #1 met")

    assert main([], tmp_path) == 0
    assert "1 complete" in capsys.readouterr().out


def test_listing_prints_every_task_without_checking(tmp_path, capsys):
    """`--list` is the reading aid used to find drift by hand."""
    write(tmp_path, "cli-99 - A-task", "Done", "- [ ] #1 open")

    assert main(["--list"], tmp_path) == 0
    assert "CLI-99\tDone\t0/1" in capsys.readouterr().out
