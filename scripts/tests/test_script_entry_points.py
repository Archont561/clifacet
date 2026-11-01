"""Every script starts when it is run the way Pixi and the workflows run it.

The tests import the scripts as `scripts.<name>`, which puts the repository root on
`sys.path` and hides a whole class of failure: run as `python scripts/<name>.py`, Python
puts `scripts/` there instead, so a sibling import raises `ModuleNotFoundError` and a
missed `import sys` is a `NameError` at first use. Both actually happened while the
release logic was moved out of the workflow, and neither was visible to the suite.

Each script is therefore started in a subprocess, from the repository root, exactly as a
task does. A script with an argument parser is asked for `--help`, which imports it and
exits before doing anything; one that takes positional arguments is started bare, and the
rule is that it fails with a message rather than a traceback.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT_DIR = ROOT / "scripts"

# Imported by scripts and by tests, but never run: it has no `main`.
LIBRARIES = {"github_actions"}
# `restore.sh` and `bootstrap.sh` are shell; `tests/` is the suite itself.
SCRIPTS = sorted(
    path for path in SCRIPT_DIR.glob("*.py") if path.stem not in LIBRARIES
)


def uses_parser(script: Path) -> bool:
    """Whether the script takes `--help`, or only positional arguments."""
    return "argparse" in script.read_text(encoding="utf-8")


def run(script: Path, *arguments: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(script), *arguments],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


@pytest.mark.parametrize("script", SCRIPTS, ids=lambda path: path.name)
def test_the_script_starts_as_a_script(script):
    completed = run(script, "--help") if uses_parser(script) else run(script)

    assert "Traceback" not in completed.stderr, (
        f"{script.name} raises when run as a script, which is how Pixi and the workflows "
        f"run it:\n{completed.stderr}"
    )
    if uses_parser(script):
        assert completed.returncode == 0
        assert "usage" in completed.stdout.lower()


def test_every_script_has_a_test_module():
    """A script with no test file is a script whose rules are nobody's job."""
    names = {script.stem for script in SCRIPTS} | LIBRARIES
    tested = {
        path.stem.removeprefix("test_")
        for path in (SCRIPT_DIR / "tests").glob("test_*.py")
    }
    # `check_advisory_exception.py` is covered by `test_advisory_exception.py`: the
    # `check_` prefix is implied by the directory it lives in.
    covered = tested | {f"check_{name}" for name in tested}

    assert names - covered == set(), "every script needs a test module of the same name"


@pytest.mark.parametrize("script", SCRIPTS, ids=lambda path: path.name)
def test_the_script_does_not_rely_on_the_caller_working_directory(script):
    """A task runs from the repository root, but a maintainer may not."""
    if not uses_parser(script):
        pytest.skip("started bare, which is not a working-directory test")
    completed = subprocess.run(
        [sys.executable, str(script), "--help"],
        cwd=SCRIPT_DIR,
        capture_output=True,
        text=True,
        check=False,
    )

    assert "Traceback" not in completed.stderr, (
        f"{script.name} only starts from the repository root:\n{completed.stderr}"
    )
