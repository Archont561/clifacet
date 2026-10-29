"""Exercise devcontainer bootstrap without downloading or installing tools."""

import json
import os
import platform
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SETUP = ROOT / ".devcontainer" / "setup.sh"
CONFIG = ROOT / ".devcontainer" / "devcontainer.json"


class DevcontainerTests(unittest.TestCase):
    def test_config_is_valid_json_and_points_to_bootstrap(self):
        config = json.loads(CONFIG.read_text(encoding="utf-8"))
        self.assertIn("ghcr.io/prefix-dev/pixi:0.83.0@sha256:", config["image"])
        self.assertEqual(config["postCreateCommand"], "bash .devcontainer/setup.sh")
        self.assertIn("/root/.bun/bin", config["remoteEnv"]["PATH"])
        self.assertTrue(SETUP.is_file())

    @unittest.skipUnless(platform.system() == "Linux" and platform.machine() == "x86_64", "linux-64 bootstrap")
    def test_default_setup_installs_pinned_project_and_agent(self):
        result, calls = self.run_setup()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("global install --channel conda-forge git gh", calls)
        self.assertIn("ar=x86_64-conda-linux-gnu-ar c-compiler", calls)
        self.assertIn("install --locked --all", calls)
        self.assertIn("run --frozen bun-install", calls)
        self.assertIn("run --frozen -x bun add --global --exact opencode-ai@1.19.5", calls)
        self.assertNotIn("models --refresh", calls)
        steps = [
            "global install --channel conda-forge git gh",
            "ar=x86_64-conda-linux-gnu-ar c-compiler",
            "install --locked --all",
            "run --frozen bun-install",
            "run --frozen -x bun add --global --exact opencode-ai@1.19.5",
        ]
        self.assertEqual(sorted(calls.index(step) for step in steps), [calls.index(step) for step in steps])

    @unittest.skipUnless(platform.system() == "Linux" and platform.machine() == "x86_64", "linux-64 bootstrap")
    def test_optional_agent_can_be_disabled(self):
        result, calls = self.run_setup(CLIFACET_INSTALL_OPENCODE="0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("run --frozen bun-install", calls)
        self.assertNotIn("opencode-ai", calls)

    @unittest.skipUnless(platform.system() == "Linux" and platform.machine() == "x86_64", "linux-64 bootstrap")
    def test_invalid_option_fails_before_install(self):
        result, calls = self.run_setup(CLIFACET_INSTALL_OPENCODE="no")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("must be 0 or 1", result.stderr)
        self.assertEqual(calls, "")

    @unittest.skipUnless(platform.system() == "Linux" and platform.machine() == "x86_64", "linux-64 bootstrap")
    def test_failed_install_stops_bootstrap(self):
        result, calls = self.run_setup(FAKE_FAIL="global")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("install --locked --all", calls)
        self.assertNotIn("bun-install", calls)

    @unittest.skipUnless(platform.system() == "Linux" and platform.machine() == "x86_64", "linux-64 bootstrap")
    def test_bun_install_failure_prevents_opencode_install(self):
        result, calls = self.run_setup(FAKE_FAIL="bun")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("run --frozen bun-install", calls)
        self.assertNotIn("opencode-ai", calls)

    def run_setup(self, **extra):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fakebin = root / "bin"
            fakebin.mkdir()
            (fakebin / "pixi").write_text(
                '#!/usr/bin/env bash\n'
                'echo "$*" >> "$FAKE_LOG"\n'
                'if [[ "$*" == "--version" ]]; then echo "pixi fake"; fi\n'
                'if [[ "$*" == *"bun pm bin -g"* ]]; then echo "$HOME/.bun/bin"; fi\n'
                'if [[ "${FAKE_FAIL:-}" == global && "$1" == global ]]; then exit 1; fi\n'
                'if [[ "${FAKE_FAIL:-}" == bun && "$*" == "run --frozen bun-install" ]]; then exit 1; fi\n',
                encoding="utf-8",
            )
            (fakebin / "pixi").chmod(0o755)
            bunbin = root / ".bun" / "bin"
            bunbin.mkdir(parents=True)
            (bunbin / "opencode").write_text("#!/bin/sh\necho 1.19.5\n", encoding="utf-8")
            (bunbin / "opencode").chmod(0o755)
            log = root / "calls"
            env = os.environ.copy()
            env.update({"HOME": str(root), "PATH": str(fakebin) + os.pathsep + env["PATH"], "FAKE_LOG": str(log)})
            env.pop("CLIFACET_INSTALL_OPENCODE", None)
            env.pop("FAKE_FAIL", None)
            env.update(extra)
            result = subprocess.run(["bash", str(SETUP)], env=env, text=True, capture_output=True, check=False)
            return result, log.read_text(encoding="utf-8") if log.exists() else ""


if __name__ == "__main__":
    unittest.main()
