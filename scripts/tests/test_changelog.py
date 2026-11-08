"""The changelog is rendered, not written: convco's output, normalised and checked."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest

from scripts.changelog import (
    ROOT,
    UNRELEASED,
    UnreleasedError,
    commit,
    convco,
    generate,
    main,
    notes,
    notes_for,
    render,
    with_unreleased,
)

RAW = """# Changelog

All notable changes are documented here.

## [Unreleased](https://example.test/compare/v0.1.0...abc) (2026-11-01)

## v0.1.0 (2026-11-01)

### ⚠ BREAKING CHANGE

* `clifacet::example` is removed.



### Added

* **prompt:** ship a dialog frontend
([a64437c](https://example.test/commit/a64437c))

### Fixed

* **ci:** restore an import
([86f1218](https://example.test/commit/86f1218))
"""

PATCH_RAW = """# Changelog

## [Unreleased](https://example.test/compare/v0.1.0...abc) (2026-11-01)

### Added

* **prompt:** ask before overwriting

## v0.1.0 (2026-11-01)

### Added

* something older
"""


def test_headings_become_keep_a_changelog_headings() -> None:
    rendered = render(RAW)
    assert "## [Unreleased]" in rendered
    assert "## [0.1.0] - 2026-11-01" in rendered
    assert "v0.1.0 (" not in rendered


def test_a_patch_section_is_promoted_to_the_same_level_as_a_minor() -> None:
    # convco renders a patch version one level deeper than a minor one; a changelog
    # reader must not have to notice that.
    rendered = render(PATCH_RAW)
    assert "## [Unreleased]" in rendered
    assert "## [0.1.0] - 2026-11-01" in rendered
    assert "### Added" in rendered
    assert "#### Added" not in rendered


def test_the_compare_link_moves_to_a_reference_at_the_bottom() -> None:
    rendered = render(RAW)
    body, _, links = rendered.partition("\n[Unreleased]: ")
    assert "](https://example.test/compare" not in body
    assert links.startswith("https://example.test/compare/v0.1.0...HEAD")


def test_unreleased_carries_neither_a_date_nor_a_commit() -> None:
    # Either one would rewrite the file on every commit, whether or not a user-visible
    # section changed, and a check that always fails is a check nobody reads.
    rendered = render(RAW)
    assert "## [Unreleased]\n" in rendered
    assert "## [Unreleased] - " not in rendered
    assert "[Unreleased]: https://example.test/compare/v0.1.0...HEAD\n" in rendered


def test_blank_line_runs_are_collapsed() -> None:
    rendered = render(RAW)
    assert "\n\n\n" not in rendered


def test_the_header_is_kept_verbatim() -> None:
    rendered = render(RAW)
    assert rendered.startswith("# Changelog\n\nAll notable changes are documented here.\n")


def test_the_file_ends_with_exactly_one_newline() -> None:
    assert render(RAW).endswith("\n")
    assert not render(RAW).endswith("\n\n")


def test_notes_take_the_unreleased_section_and_name_it_with_the_version() -> None:
    released = notes(PATCH_RAW, "0.2.0", "2026-12-01")
    assert released.startswith("## [0.2.0] - 2026-12-01\n")
    assert "ask before overwriting" in released
    assert "something older" not in released


def test_notes_refuse_a_release_with_nothing_to_say() -> None:
    empty = """# Changelog

## [Unreleased](https://example.test/compare/v0.1.0...abc) (2026-11-01)

## v0.1.0 (2026-11-01)

### Added

* something older
"""
    with pytest.raises(UnreleasedError, match="nothing to release"):
        notes(empty, "0.2.0", "2026-12-01")


def test_notes_carry_no_compare_link_that_cannot_exist_yet() -> None:
    released = notes(PATCH_RAW, "0.2.0", "2026-12-01")
    assert "](https://example.test/compare" not in released


def test_main_print_writes_rendered_output(tmp_path, capsys) -> None:
    (tmp_path / "CHANGELOG.md").write_text("stale\n", encoding="utf-8")
    code = main(["print"], root=tmp_path, runner=lambda argv: (0, RAW))
    assert code == 0
    assert "## [0.1.0] - 2026-11-01" in capsys.readouterr().out


def test_main_regen_writes_the_file(tmp_path) -> None:
    target = tmp_path / "CHANGELOG.md"
    target.write_text("stale\n", encoding="utf-8")
    code = main(["regen"], root=tmp_path, runner=lambda argv: (0, RAW))
    assert code == 0
    assert "## [0.1.0] - 2026-11-01" in target.read_text(encoding="utf-8")


def test_main_check_passes_when_the_file_matches(tmp_path) -> None:
    target = tmp_path / "CHANGELOG.md"
    target.write_text(render(RAW), encoding="utf-8")
    assert main(["check"], root=tmp_path, runner=lambda argv: (0, RAW)) == 0


def test_main_check_fails_when_the_file_is_stale(tmp_path, capsys) -> None:
    (tmp_path / "CHANGELOG.md").write_text("stale\n", encoding="utf-8")
    assert main(["check"], root=tmp_path, runner=lambda argv: (0, RAW)) == 1
    assert "pixi run changelog" in capsys.readouterr().err


def test_main_reports_a_convco_failure(tmp_path, capsys) -> None:
    code = main(["print"], root=tmp_path, runner=lambda argv: (2, "boom"))
    assert code == 1
    assert "convco" in capsys.readouterr().err


# --- committing the regenerated file -------------------------------------------


def committing_argv(identity=None):
    """The `git commit` invocation `commit` makes, with git and the push stood in for."""
    calls = []

    def run(argv):
        calls.append(list(argv))
        return 0, (" M CHANGELOG.md\n" if "status" in argv else "")

    commit(Path("."), "main", run=run, push=lambda argv: (0, ""), identity=identity)
    return next(call for call in calls if "commit" in call)


def test_commit_names_the_identity_it_is_given():
    """A fresh runner has no git identity: the 0.3.0 release's changelog step failed with
    `Author identity unknown` after the tag and both crates had shipped."""
    argv = committing_argv(identity=("release-bot", "bot@example.test"))

    assert argv[:6] == [
        "git",
        "-c",
        "user.name=release-bot",
        "-c",
        "user.email=bot@example.test",
        "commit",
    ]


def test_commit_without_an_identity_uses_the_configured_one():
    """A maintainer running `changelog commit` locally commits as themselves."""
    assert committing_argv()[:2] == ["git", "commit"]



# The release renders CHANGELOG.md while HEAD is the tag it just created, then commits
# the file — and that commit moves HEAD past the tag. These run the real convco against
# a real history, because the bug lives in what convco emits at a tag, not in any fake.

_GIT_ENVIRONMENT = ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR")


def _git(repository: Path, *arguments: str) -> None:
    subprocess.run(["git", *arguments], cwd=repository, check=True, capture_output=True)


def _history(repository: Path, monkeypatch, tags: list[str]) -> Path:
    """A history with one describable commit per tag, HEAD exactly on the last tag."""
    # A hook that runs this suite exports the outer repository's git variables; convco and
    # git would follow them out of the temporary history.
    for name in _GIT_ENVIRONMENT:
        monkeypatch.delenv(name, raising=False)
    _git(repository, "init", "-q", "-b", "main")
    _git(repository, "config", "user.name", "Test")
    _git(repository, "config", "user.email", "test@example.com")
    shutil.copy(ROOT / ".versionrc", repository / ".versionrc")
    _git(repository, "add", ".versionrc")
    for number, tag in enumerate(tags):
        _git(repository, "commit", "-q", "--allow-empty", "-m", f"feat: feature {number}")
        _git(repository, "tag", tag)
    return repository


@pytest.fixture
def released(tmp_path, monkeypatch) -> Path:
    """The moment after a release's tag: HEAD is `v0.1.1`, which follows `v0.1.0`."""
    return _history(tmp_path, monkeypatch, ["v0.1.0", "v0.1.1"])


@pytest.mark.parametrize("hidden", ["chore(release): regenerate the changelog", "docs: reword"])
def test_a_render_at_the_tag_survives_the_commit_that_records_it(released, hidden) -> None:
    at_tag = generate(convco(released), released)
    _git(released, "commit", "-q", "--allow-empty", "-m", hidden)
    assert generate(convco(released), released) == at_tag


def test_the_first_release_survives_the_commit_that_records_it(tmp_path, monkeypatch) -> None:
    # The first version has no compare link of its own to borrow the repository from.
    repository = _history(tmp_path, monkeypatch, ["v0.1.0"])
    at_tag = generate(convco(repository), repository)
    _git(repository, "commit", "-q", "--allow-empty", "-m", "chore: tidy")
    assert generate(convco(repository), repository) == at_tag


def test_the_stable_heading_is_empty_and_compares_the_tag_with_head(released) -> None:
    rendered = generate(convco(released), released)
    assert "## [Unreleased]\n\n## [0.1.1] - " in rendered
    assert (
        "[Unreleased]: https://github.com/Archont561/clifacet/compare/v0.1.1...HEAD"
        in rendered.splitlines()
    )


def test_release_notes_never_carry_the_unreleased_heading(released) -> None:
    _git(released, "commit", "-q", "--allow-empty", "-m", "feat: the next feature")
    text = notes_for(convco(released), "0.2.0", "2026-11-09")
    assert text.startswith("## [0.2.0] - 2026-11-09\n")
    assert UNRELEASED not in text


def test_an_unreleased_section_convco_wrote_is_left_alone() -> None:
    assert with_unreleased(RAW) == RAW
