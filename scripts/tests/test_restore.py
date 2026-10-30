"""The generated sandbox restore launcher fails closed."""

import os
import subprocess
from pathlib import Path

RESTORE = Path(__file__).resolve().parents[1] / "restore.sh"

POLICY = """schema = 1
branch_prefix = "sandbox"
[[bundle]]
name = "developer"
environments = ["default"]
platforms = ["linux-64"]
"""


def test_missing_branch_without_fetch_fails_without_touching_the_environment(linux_64, tmp_path):
    subprocess.run(["git", "init", "-q", str(tmp_path)], check=True)
    (tmp_path / "pixi-sandbox.toml").write_text(POLICY, encoding="utf-8")

    env = {**os.environ, "HOME": str(tmp_path), "PIXI_SANDBOX_FETCH": "skip"}
    env.pop("PIXI_SANDBOX_BRANCH", None)
    env.pop("PIXI_SANDBOX_BUNDLE", None)

    result = subprocess.run(
        ["sh", str(RESTORE)], cwd=tmp_path, env=env, text=True, capture_output=True, check=False
    )

    assert result.returncode == 2
    assert "sandbox/developer-linux-64" in result.stderr
    assert "PIXI_SANDBOX_FETCH=skip" in result.stderr
    assert not (tmp_path / ".pixi").exists()
    assert not (tmp_path / ".local").exists()
