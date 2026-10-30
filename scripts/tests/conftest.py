"""Shared fixtures for the repository script tests.

This file also puts the repository root on `sys.path`, so the suites can import
`scripts.*` as a namespace package regardless of the directory pytest runs from.
"""

import platform
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.check_skills import REQUIRED  # noqa: E402  (needs the path above)

SKILLS = ROOT / ".agents" / "skills"


@pytest.fixture(scope="session")
def repo_root():
    """The repository root, for tests that validate the committed tree."""
    return ROOT


@pytest.fixture(scope="session")
def linux_64():
    """Skip unless the committed sandbox bundle can run on this host."""
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        pytest.skip("the committed sandbox bundle targets linux-64")


@pytest.fixture
def skills_root(tmp_path):
    """A skills tree holding every required skill with valid frontmatter."""
    for name in REQUIRED:
        directory = tmp_path / ".agents" / "skills" / name
        directory.mkdir(parents=True)
        (directory / "SKILL.md").write_text(
            f"---\nname: {name}\ndescription: A useful skill.\n---\n\n# Skill\n",
            encoding="utf-8",
        )
    return tmp_path


def skill_file(root, name):
    """Path to one skill's `SKILL.md` inside a fixture tree."""
    return Path(root) / ".agents" / "skills" / name / "SKILL.md"
