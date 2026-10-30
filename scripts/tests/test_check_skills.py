"""Required local skill files and frontmatter validation."""

import pytest

from conftest import skill_file
from scripts.check_skills import check_skills

INVALID_FRONTMATTER = [
    pytest.param(
        "---\nname: other\ndescription: A useful skill.\n---\n\n# Skill\n",
        "name does not match",
        id="name-mismatch",
    ),
    pytest.param("---\nname: session\n---\n\n# Skill\n", "missing description", id="no-description"),
    pytest.param("", "invalid frontmatter or empty body", id="empty-file"),
    pytest.param("---\nname: session\n---\n", "invalid frontmatter or empty body", id="empty-body"),
]


def test_valid_tree_has_no_problems(skills_root):
    assert check_skills(skills_root) == []


def test_missing_skill_file_is_reported(skills_root):
    skill_file(skills_root, "session").unlink()

    problems = check_skills(skills_root)

    assert any("session" in problem for problem in problems), problems


@pytest.mark.parametrize(("content", "expected"), INVALID_FRONTMATTER)
def test_invalid_frontmatter_is_reported(skills_root, content, expected):
    skill_file(skills_root, "session").write_text(content, encoding="utf-8")

    problems = check_skills(skills_root)

    assert any(expected in problem for problem in problems), problems


def test_symlinked_skill_is_rejected(skills_root):
    link = skill_file(skills_root, "session")
    link.unlink()
    link.symlink_to(skill_file(skills_root, "backlog"))

    problems = check_skills(skills_root)

    assert any("symlinked" in problem for problem in problems), problems


def test_committed_repository_is_valid(repo_root):
    assert check_skills(repo_root) == []
