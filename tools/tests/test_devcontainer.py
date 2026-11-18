"""Devcontainer bootstrap, exercised without downloading or installing tools."""

import json
import os
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SETUP = ROOT / ".devcontainer" / "setup.sh"
CONFIG = ROOT / ".devcontainer" / "devcontainer.json"

# Records every invocation and reproduces the three failures the suite scripts.
PIXI_STUB = """#!/usr/bin/env bash
echo "$*" >> "$FAKE_LOG"
if [[ "$*" == "--version" ]]; then echo "pixi fake"; fi
if [[ "$*" == *"bun pm bin -g"* ]]; then echo "$HOME/.bun/bin"; fi
if [[ "${FAKE_FAIL:-}" == global && "$1" == global ]]; then exit 1; fi
if [[ "${FAKE_FAIL:-}" == bun && "$*" == "run --frozen bun-install" ]]; then exit 1; fi
"""

# The bootstrap's steps, in the order the suite requires them.
STEPS = (
    "global install --channel conda-forge git gh",
    "ar=x86_64-conda-linux-gnu-ar c-compiler",
    "install --locked --all",
    "run --frozen bun-install",
    "run --frozen -x bun add --global --exact opencode-ai@1.19.5",
)


@pytest.fixture(scope="module")
def config():
    return json.loads(CONFIG.read_text(encoding="utf-8"))


@pytest.fixture
def run_setup(linux_64, tmp_path):
    """Run the bootstrap against stubbed tools, returning the result and its call log."""
    fakebin = tmp_path / "bin"
    fakebin.mkdir()
    pixi = fakebin / "pixi"
    pixi.write_text(PIXI_STUB, encoding="utf-8")
    pixi.chmod(0o755)

    bunbin = tmp_path / ".bun" / "bin"
    bunbin.mkdir(parents=True)
    opencode = bunbin / "opencode"
    opencode.write_text("#!/bin/sh\necho 1.19.5\n", encoding="utf-8")
    opencode.chmod(0o755)

    log = tmp_path / "calls"

    def run(**extra):
        env = {**os.environ, "HOME": str(tmp_path), "FAKE_LOG": str(log)}
        env["PATH"] = os.pathsep.join([str(fakebin), env["PATH"]])
        env.pop("CLIFACET_INSTALL_OPENCODE", None)
        env.pop("FAKE_FAIL", None)
        env.update(extra)
        result = subprocess.run(
            ["bash", str(SETUP)], env=env, text=True, capture_output=True, check=False
        )
        return result, log.read_text(encoding="utf-8") if log.exists() else ""

    return run


def test_config_is_valid_json_and_points_to_the_bootstrap(config):
    assert "ghcr.io/prefix-dev/pixi:0.83.0@sha256:" in config["image"]
    assert config["postCreateCommand"] == "bash .devcontainer/setup.sh"
    assert "/root/.bun/bin" in config["remoteEnv"]["PATH"]
    assert SETUP.is_file()


def test_default_setup_installs_pinned_project_and_agent(run_setup):
    result, calls = run_setup()

    assert result.returncode == 0, result.stderr
    for step in STEPS:
        assert step in calls
    assert "models --refresh" not in calls
    positions = [calls.index(step) for step in STEPS]
    assert positions == sorted(positions), calls


def test_optional_agent_can_be_disabled(run_setup):
    result, calls = run_setup(CLIFACET_INSTALL_OPENCODE="0")

    assert result.returncode == 0, result.stderr
    assert "run --frozen bun-install" in calls
    assert "opencode-ai" not in calls


def test_invalid_option_fails_before_install(run_setup):
    result, calls = run_setup(CLIFACET_INSTALL_OPENCODE="no")

    assert result.returncode != 0
    assert "must be 0 or 1" in result.stderr
    assert calls == ""


def test_failed_install_stops_bootstrap(run_setup):
    result, calls = run_setup(FAKE_FAIL="global")

    assert result.returncode != 0
    assert "install --locked --all" not in calls
    assert "bun-install" not in calls


def test_bun_install_failure_prevents_opencode_install(run_setup):
    result, calls = run_setup(FAKE_FAIL="bun")

    assert result.returncode != 0
    assert "run --frozen bun-install" in calls
    assert "opencode-ai" not in calls
