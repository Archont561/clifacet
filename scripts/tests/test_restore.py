"""Fail-closed checks for the generated sandbox restore launcher."""

import os
import platform
import subprocess
import tempfile
import unittest
from pathlib import Path

RESTORE = Path(__file__).resolve().parents[1] / "restore.sh"


class RestoreTests(unittest.TestCase):
    @unittest.skipUnless(
        platform.system() == "Linux" and platform.machine() == "x86_64",
        "the committed sandbox bundle currently targets linux-64",
    )
    def test_no_fetch_and_missing_branch_fails_without_modifying_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            (root / "pixi-sandbox.toml").write_text(
                'schema = 1\nbranch_prefix = "sandbox"\n'
                '[[bundle]]\nname = "developer"\n'
                'environments = ["default"]\nplatforms = ["linux-64"]\n',
                encoding="utf-8",
            )
            env = os.environ.copy()
            env.update({"HOME": directory, "PIXI_SANDBOX_FETCH": "skip"})
            env.pop("PIXI_SANDBOX_BRANCH", None)
            env.pop("PIXI_SANDBOX_BUNDLE", None)
            result = subprocess.run(
                ["sh", str(RESTORE)], cwd=root, env=env, text=True, capture_output=True, check=False
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("sandbox/developer-linux-64", result.stderr)
            self.assertIn("PIXI_SANDBOX_FETCH=skip", result.stderr)
            self.assertFalse((root / ".pixi").exists())
            self.assertFalse((root / ".local").exists())


if __name__ == "__main__":
    unittest.main()
