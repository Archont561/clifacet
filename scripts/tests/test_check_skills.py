"""Tests for required local skill files and frontmatter validation."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "check_skills.py"
spec = importlib.util.spec_from_file_location("check_skills", SCRIPT)
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class SkillChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for name in checker.REQUIRED:
            directory = self.root / ".agents" / "skills" / name
            directory.mkdir(parents=True)
            (directory / "SKILL.md").write_text(
                f"---\nname: {name}\ndescription: A useful skill.\n---\n\n# Skill\n",
                encoding="utf-8",
            )

    def path(self, name):
        return self.root / ".agents" / "skills" / name / "SKILL.md"

    def test_valid_skills(self):
        self.assertEqual(checker.check_skills(self.root), [])

    def test_missing_session_is_detected(self):
        self.path("session").unlink()
        self.assertTrue(any("session" in error for error in checker.check_skills(self.root)))

    def test_wrong_name_and_missing_description_are_detected(self):
        self.path("session").write_text("---\nname: other\n---\n\nBody\n", encoding="utf-8")
        errors = checker.check_skills(self.root)
        self.assertTrue(any("name does not match" in error for error in errors))
        self.assertTrue(any("missing description" in error for error in errors))

    def test_symlinked_local_skill_is_rejected(self):
        path = self.path("session")
        path.unlink()
        path.symlink_to(self.path("backlog"))
        self.assertTrue(any("symlinked" in error for error in checker.check_skills(self.root)))

    def test_real_repository(self):
        self.assertEqual(checker.check_skills(SCRIPT.parents[1]), [])


if __name__ == "__main__":
    unittest.main()
