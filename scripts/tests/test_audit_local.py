"""The local-audit wrapper steps aside only when it honestly cannot audit."""

from pathlib import Path

import pytest

from scripts.audit_local import audit_command, database_path, database_ready, decide


def test_database_follows_cargo_home_when_it_is_set(tmp_path):
    """A restored sandbox redirects CARGO_HOME, so the clone lives there too."""
    assert database_path(str(tmp_path / "cargo-home"), tmp_path) == (
        tmp_path / "cargo-home" / "advisory-db"
    )


def test_database_defaults_to_the_cargo_audit_location(tmp_path):
    assert database_path(None, tmp_path) == tmp_path / ".cargo" / "advisory-db"
    assert database_path("", tmp_path) == tmp_path / ".cargo" / "advisory-db"


def test_an_empty_database_directory_is_not_ready(tmp_path):
    (tmp_path / "advisory-db").mkdir()
    assert database_ready(tmp_path / "advisory-db") is False


def test_a_cloned_database_is_ready(tmp_path):
    (tmp_path / "advisory-db" / "crates").mkdir(parents=True)
    assert database_ready(tmp_path / "advisory-db") is True


@pytest.mark.parametrize(
    ("ready", "require", "expected"),
    [
        (True, False, "run"),
        (True, True, "run"),
        (False, False, "skip"),
        (False, True, "fail"),
    ],
)
def test_the_decision_never_silently_passes_a_required_audit(ready, require, expected):
    assert decide(ready, require) == expected


def test_the_command_uses_the_local_clone_and_tolerates_a_stale_one(tmp_path):
    command = audit_command(tmp_path / "advisory-db")
    assert "--no-fetch" in command
    assert "--stale" in command
    assert str(tmp_path / "advisory-db") in command
    assert command.index("--deny") + 1 == command.index("warnings")


def test_a_missing_database_skips_without_running_cargo(monkeypatch, tmp_path, capsys):
    from scripts import audit_local

    monkeypatch.setenv("CARGO_HOME", str(tmp_path / "cargo-home"))
    monkeypatch.setattr(audit_local.subprocess, "run", _forbidden_run)

    assert audit_local.main([]) == 0
    assert "is skipped" in capsys.readouterr().err


def test_a_required_missing_database_fails_without_running_cargo(monkeypatch, tmp_path, capsys):
    from scripts import audit_local

    monkeypatch.setenv("CARGO_HOME", str(tmp_path / "cargo-home"))
    monkeypatch.setattr(audit_local.subprocess, "run", _forbidden_run)

    assert audit_local.main(["--require"]) == 1
    assert "--require was given" in capsys.readouterr().err


def test_a_present_database_runs_the_audit_and_forwards_its_status(monkeypatch, tmp_path):
    from scripts import audit_local

    database = tmp_path / "cargo-home" / "advisory-db"
    (database / "crates").mkdir(parents=True)
    monkeypatch.setenv("CARGO_HOME", str(tmp_path / "cargo-home"))
    calls = []

    def record(command, cwd, check):
        calls.append((command, cwd))
        return type("Result", (), {"returncode": 7})()

    monkeypatch.setattr(audit_local.subprocess, "run", record)

    assert audit_local.main([]) == 7
    assert calls == [(audit_command(database), Path(audit_local.ROOT))]


def _forbidden_run(*args, **kwargs):
    raise AssertionError("cargo must not run when there is nothing to audit against")
