"""The bridge from doit to a script's `main`, and the graph that uses it.

`dodo.py` is a registry, so almost every way it can be wrong is a way of calling something
wrongly — and the failures are quiet. A task that returns a `main`'s exit code is recorded
by doit as a success. A task that passes `None` as `argv` makes `argparse` read doit's own
command line, so the script fails with an argument error naming a subcommand it never had.
Both happened while this file's package was being written, and neither was visible from
the task list, which is why they are checked here rather than read.

The graph itself is checked too: every task must be discoverable, every task must say what
it covers, and the gate must not include a task that fetches.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

from automation.lib.tasks import run

ROOT = Path(__file__).resolve().parents[2]
DODO = ROOT / "dodo.py"


def tasks() -> dict:
    """The task functions `dodo.py` declares, by the name doit gives them."""
    namespace: dict = {"__name__": "dodo"}
    exec(compile(DODO.read_text(encoding="utf-8"), str(DODO), "exec"), namespace)  # noqa: S102
    return {
        name.removeprefix("task_"): value
        for name, value in namespace.items()
        if name.startswith("task_") and callable(value)
    }


def test_the_bridge_letting_a_script_run_is_not_silent():
    """`main` returning 1 must raise, because doit reads the return value as the result.

    `check_coverage` exits non-zero when coverage is under the floor; a task that treated
    that as a result would report a passing gate.
    """
    with pytest.raises(RuntimeError) as failure:
        run("check_coverage", argv=["/nonexistent/lcov.info"])

    assert "exited" in str(failure.value)


def test_a_script_without_argv_is_called_bare():
    """`check_skills.main` takes nothing; passing it an argv would be a `TypeError`."""
    assert run("check_skills") is None


def test_an_argv_is_always_passed_when_the_script_takes_one():
    """`None` would make `argparse` read `sys.argv` — doit's own command line.

    `check_docs --help` is the cheapest script that proves the argument arrived: argparse
    prints its usage and exits, which the bridge must treat as the script's own exit.
    """
    with pytest.raises(SystemExit) as exit_info:
        run("check_docs", argv=["--help"])

    assert exit_info.value.code == 0


def test_the_script_lookup_names_a_module_that_exists():
    from automation.lib.tasks import main  # noqa: PLC0415 — the name is the point

    assert callable(main("check_docs"))


def test_the_lookup_of_a_missing_script_names_it(monkeypatch):
    monkeypatch.setattr("automation.lib.tasks.SCRIPTS", "no_such_package")

    with pytest.raises(ModuleNotFoundError):
        run("anything")


def test_every_task_declares_what_it_covers():
    """A task with no docstring is a task nobody can choose from `automation list`."""
    undocumented = sorted(name for name, build in tasks().items() if not build.__doc__)

    assert not undocumented, f"every task needs a docstring; these have none: {undocumented}"


def test_every_task_is_built_by_doit_without_arguments():
    """`dodo.py` must not execute anything at import: doit calls each builder itself."""
    for name, build in tasks().items():
        task = build()
        assert isinstance(task, dict), f"{name} must return a dict"
        assert "actions" in task, f"{name} must declare its actions"


def test_a_task_with_actions_declares_at_least_one():
    for name, build in tasks().items():
        task = build()
        if task.get("task_dep"):
            continue
        assert task["actions"], f"{name} has neither actions nor dependencies"


def test_every_dependency_names_a_real_task():
    known = set(tasks())

    for name, build in tasks().items():
        for dependency in build().get("task_dep", []):
            assert dependency in known, f"{name} depends on {dependency}, which does not exist"


def test_the_gate_does_not_fetch():
    """`gates` runs after a restore, where there is no network.

    `audit` and `deny` fetch the RustSec database, `readme_check` fetches a package from
    npm, and `backlog_check` needs a network install of the bun packages. All of them
    belong to a person's command or to CI, not to the gate a contributor runs offline.

    `markdownlint` is deliberately absent from that set: its tool is pinned in
    `package.json` and installed by `bun-install` before a session starts, so the check
    reads `node_modules` and never fetches.
    """
    needs_network = {"audit", "deny", "readme_check", "backlog_check"}

    assert not set(tasks()["gates"]().get("task_dep", [])) & needs_network


def test_the_gate_runs_the_documentation_check():
    """Repo policy: a guide is checked by the gate that owns documentation."""
    dependencies = set(tasks()["gates"]().get("task_dep", []))

    assert "docs" in dependencies


def test_the_gate_runs_the_tests():
    assert "test" in set(tasks()["gates"]().get("task_dep", []))


def test_the_online_gate_is_the_gate_plus_the_rustsec_checks():
    dependencies = set(tasks()["gates_online"]().get("task_dep", []))

    assert {"gates", "audit", "deny"} <= dependencies


def test_no_task_declares_a_target_that_would_hide_a_failure():
    """A `targets` entry lets doit skip an action when the file is newer than its inputs.

    Every check here answers a question about the tree, so a skipped check is a check that
    reports success without looking. `docs_write` and `changelog` write, but they write on
    purpose rather than caching, so they declare no targets either.
    """
    for name, build in tasks().items():
        assert "targets" not in build(), f"{name} declares targets, so doit could skip it"
        assert "file_dep" not in build(), f"{name} declares file_dep, so doit could skip it"


def test_the_task_list_runs_through_the_cli():
    """`pixi run automation list` is the discovery command; it must not need a network."""
    finished = subprocess.run(
        [sys.executable, "-m", "doit", "list"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )

    assert finished.returncode == 0, finished.stderr
    assert "gates" in finished.stdout
