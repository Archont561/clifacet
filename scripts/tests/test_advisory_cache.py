"""The advisory-database cache: one key per ISO week, and clones that survive it.

The database is cached so `cargo audit` does not re-clone RustSec on every push. Two
pieces of logic made that cache work, both of them in YAML:

- an ISO week key, so a restore-keys fallback can hand over last week's clone;
- a reset of whatever the cache restored, because a clone left dirty by a cancelled run
  makes the next audit fail for a reason that has nothing to do with the code.

The week key is worth testing rather than trusting: `%V`/`%G` are ISO week and ISO
week-year, and the week-year is not the calendar year at the turn of a year.
"""

from __future__ import annotations

import datetime
import shutil
import subprocess

import pytest

from scripts.advisory_cache import cache_key, clean, default_paths, main


def test_the_key_is_the_iso_year_and_week():
    assert cache_key(datetime.date(2026, 11, 1)) == "2026-W44"


def test_the_week_is_zero_padded_so_it_sorts_and_matches():
    assert cache_key(datetime.date(2026, 1, 5)) == "2026-W02"


def test_the_year_in_the_key_is_the_iso_week_year_not_the_calendar_year():
    """2027-01-01 belongs to ISO week 53 of 2026; using the calendar year would clash."""
    assert cache_key(datetime.date(2027, 1, 1)) == "2026-W53"


def test_a_clone_is_reset_and_cleaned_before_use(tmp_path):
    seen = []
    clone = tmp_path / "advisory-db"
    (clone / ".git").mkdir(parents=True)

    cleaned = clean([clone], run=seen.append)

    assert cleaned == [clone]
    assert seen == [
        ["git", "-C", str(clone), "reset", "--hard", "HEAD"],
        ["git", "-C", str(clone), "clean", "-ffdx"],
    ]


def test_a_directory_that_is_not_a_clone_is_left_alone(tmp_path):
    seen = []
    plain = tmp_path / "advisory-db"
    plain.mkdir()

    cleaned = clean([plain], run=seen.append)

    assert cleaned == []
    assert seen == []


def test_every_cached_clone_is_cleaned_not_just_the_first(tmp_path):
    seen = []
    for name in ("advisory-db", "advisory-dbs/cargo-audit"):
        (tmp_path / name / ".git").mkdir(parents=True)

    cleaned = clean([tmp_path / "advisory-db", tmp_path / "advisory-dbs" / "cargo-audit"], run=seen.append)

    assert len(cleaned) == 2
    assert len(seen) == 4


def test_paths_that_do_not_exist_are_ignored(tmp_path):
    """The cache miss case: nothing restored, nothing to clean, and no error."""
    assert clean([tmp_path / "absent"], run=lambda argv: 0) == []


def test_the_default_paths_cover_both_layouts_a_cargo_version_may_use(tmp_path, monkeypatch):
    """Cargo has used `$CARGO_HOME/advisory-db` and `$CARGO_HOME/advisory-dbs/<name>`."""
    (tmp_path / "advisory-db").mkdir()
    (tmp_path / "advisory-dbs" / "one" / ".git").mkdir(parents=True)
    (tmp_path / "advisory-dbs" / "two" / ".git").mkdir(parents=True)

    paths = default_paths(cargo_home=tmp_path)

    assert tmp_path / "advisory-db" in paths
    assert {path.name for path in paths} >= {"advisory-db", "one", "two"}


def test_the_default_paths_fall_back_to_the_home_directory(tmp_path):
    paths = default_paths(cargo_home=None, home=tmp_path)

    assert tmp_path / ".cargo" / "advisory-db" in paths


def test_clean_reaches_the_real_runner_by_default(tmp_path):
    """The injected runner never touches `_run`, which is what the workflow calls.

    A missing import there is invisible to every other test here and fails on a runner
    that has a restored clone — exactly how `import subprocess` was lost once.
    """
    if shutil.which("git") is None:
        pytest.skip("git is not available")

    clone = tmp_path / "advisory-db"
    clone.mkdir()
    _git(clone, "init", "--quiet")
    (clone / "entry.txt").write_text("clean\n", encoding="utf-8")
    _git(clone, "add", "entry.txt")
    _git(clone, "-c", "user.email=t@example.com", "-c", "user.name=t", "commit", "-qm", "seed")
    (clone / "entry.txt").write_text("dirty\n", encoding="utf-8")
    (clone / "untracked.txt").write_text("junk\n", encoding="utf-8")

    cleaned = clean([clone])

    assert cleaned == [clone]
    assert (clone / "entry.txt").read_text(encoding="utf-8") == "clean\n"
    assert not (clone / "untracked.txt").exists(), "`clean -ffdx` removes untracked files"


def _git(directory, *arguments):
    subprocess.run(["git", "-C", str(directory), *arguments], check=True, capture_output=True)


def test_main_writes_the_week_key_as_a_step_output(tmp_path, capsys):
    output = tmp_path / "output"

    assert main(["key"], today=datetime.date(2026, 11, 1), output=output) == 0
    assert output.read_text(encoding="utf-8") == "week=2026-W44\n"


def test_main_cleans_and_reports_what_it_cleaned(tmp_path, capsys):
    clone = tmp_path / "advisory-db"
    (clone / ".git").mkdir(parents=True)
    seen = []

    assert main(["clean", "--cargo-home", str(tmp_path)], run=seen.append) == 0
    assert "advisory-db" in capsys.readouterr().out
    assert seen[0][:4] == ["git", "-C", str(clone), "reset"]


def test_main_says_so_when_there_was_nothing_to_clean(tmp_path, capsys):
    assert main(["clean", "--cargo-home", str(tmp_path)], run=lambda argv: 0) == 0
    assert "no restored advisory" in capsys.readouterr().out
