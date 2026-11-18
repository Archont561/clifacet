"""Tests for Tasknest configuration and tool definitions under tools/."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def test_tasknest_check_passes():
    """`python -B -m tasknest check` must validate all discovered tasks and dependencies."""
    proc = subprocess.run(
        [sys.executable, "-B", "-m", "tasknest", "check"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, f"tasknest check failed:\n{proc.stderr}\n{proc.stdout}"
    assert "validated" in proc.stdout


def test_tasknest_ls_lists_tasks():
    """`python -B -m tasknest ls` lists discovered tasks without requiring network."""
    proc = subprocess.run(
        [sys.executable, "-B", "-m", "tasknest", "ls"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, f"tasknest ls failed:\n{proc.stderr}\n{proc.stdout}"
    assert "check" in proc.stdout
    assert "test" in proc.stdout
    assert "rust.fmt" in proc.stdout
    assert "rust.lint" in proc.stdout
    assert "repo.alint" in proc.stdout


def test_tasknest_plan_check():
    """`python -B -m tasknest plan check` produces a sequential execution plan."""
    proc = subprocess.run(
        [sys.executable, "-B", "-m", "tasknest", "plan", "check"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, f"tasknest plan failed:\n{proc.stderr}\n{proc.stdout}"
    assert "rust.fmt" in proc.stdout
    assert "repo.alint" in proc.stdout
    assert "check" in proc.stdout


def test_tasknest_manifest_autodiscovery():
    """Tasknest must autodiscover manifest-declared tasks from package.json."""
    proc = subprocess.run(
        [sys.executable, "-B", "-m", "tasknest", "ls", "-p", "root"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, f"tasknest ls -p root failed:\n{proc.stderr}\n{proc.stdout}"
    assert "root.agents-md" in proc.stdout
    assert "root.readme" in proc.stdout
    assert "root.test" in proc.stdout
    assert "root.lint" in proc.stdout
    assert "root.format" in proc.stdout


def test_tasknest_cross_package_each_selection():
    """Tasknest must plan tasks across manifest and file-based tasks using `--each`."""
    proc = subprocess.run(
        [sys.executable, "-B", "-m", "tasknest", "plan", "--each", "test"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, f"tasknest plan --each test failed:\n{proc.stderr}\n{proc.stdout}"
    assert "root.test" in proc.stdout
    assert "rust.test" in proc.stdout
