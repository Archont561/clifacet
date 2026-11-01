"""The two GitHub Actions protocols a script has to speak.

A script that replaces a shell step still has to hand values to the next step and
compose the run summary. Both are file protocols, and both have one sharp edge: an
output whose value contains a newline needs a delimiter instead of `name=value`.

Locally there is no `GITHUB_OUTPUT`, and a script that silently dropped its output
there would be untestable by hand, so the fallbacks print instead.
"""

from __future__ import annotations

import pytest

from scripts.github_actions import append_summary, set_error, set_output


def test_a_single_line_output_is_appended_in_the_plain_form(tmp_path):
    path = tmp_path / "output"

    set_output("version", "0.1.0", path=path)

    assert path.read_text(encoding="utf-8") == "version=0.1.0\n"


def test_a_second_output_is_appended_rather_than_overwriting_the_first(tmp_path):
    path = tmp_path / "output"

    set_output("version", "0.1.0", path=path)
    set_output("notes_path", "/tmp/notes.md", path=path)

    assert path.read_text(encoding="utf-8").splitlines() == [
        "version=0.1.0",
        "notes_path=/tmp/notes.md",
    ]


def test_a_multiline_output_uses_a_heredoc_delimiter(tmp_path):
    """`name=value` cannot carry a newline; GitHub's other form can."""
    path = tmp_path / "output"

    set_output("notes", "### Added\n\n- a thing", path=path)

    lines = path.read_text(encoding="utf-8").splitlines()
    assert lines[0] == "notes<<EOF_notes"
    assert lines[1:3] == ["### Added", ""]
    assert lines[-1] == "EOF_notes"


def test_without_a_github_output_file_the_value_is_printed(tmp_path, monkeypatch, capsys):
    """A local run has to show what it would have passed on."""
    monkeypatch.delenv("GITHUB_OUTPUT", raising=False)

    set_output("version", "0.1.0")

    assert capsys.readouterr().out.strip() == "version=0.1.0"


def test_the_output_file_is_read_from_the_environment_when_no_path_is_given(tmp_path, monkeypatch):
    path = tmp_path / "output"
    monkeypatch.setenv("GITHUB_OUTPUT", str(path))

    set_output("version", "0.1.0")

    assert path.read_text(encoding="utf-8") == "version=0.1.0\n"


def test_each_summary_section_ends_with_exactly_one_blank_line(tmp_path):
    """Two appends have to read as two sections, not as one run-together block."""
    path = tmp_path / "summary"

    append_summary("### Dry run", path=path)
    append_summary("| crate | state |", path=path)

    assert path.read_text(encoding="utf-8") == "### Dry run\n\n| crate | state |\n\n"


def test_without_a_summary_file_the_markdown_is_printed(monkeypatch, capsys):
    monkeypatch.delenv("GITHUB_STEP_SUMMARY", raising=False)

    append_summary("### Dry run")

    assert "### Dry run" in capsys.readouterr().out


def test_an_error_uses_the_annotation_syntax(capsys):
    set_error("this release has no notes")

    captured = capsys.readouterr()
    assert captured.err.strip() == "::error::this release has no notes"


@pytest.mark.parametrize("value", ["", "0.1.0", "refs/heads/main"])
def test_plain_values_never_use_the_delimiter_form(tmp_path, value):
    path = tmp_path / "output"

    set_output("value", value, path=path)

    assert path.read_text(encoding="utf-8") == f"value={value}\n"
