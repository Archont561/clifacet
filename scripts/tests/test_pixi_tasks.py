"""The Pixi task surface: which gates are offline-safe, and what each one covers.

`test-local` is presented everywhere — here, `AGENTS.md`, the guides — as the gate a
contributor runs with `--no-install` after a restore, and a restore has no network. A
task that silently fetches the advisory database or re-solves the environment would
therefore fail for the wrong reason, at the worst moment: on a warm checkout that
looks like the contributor's own change broke something.

That is a claim about a dependency graph, so it is checked rather than remembered.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
PIXI = ROOT / "pixi.toml"

# Tasks that need the network. `deny` and `audit` fetch the RustSec database; the
# `bun-install` pair re-solves the JavaScript dependencies and `backlog`/`skills` build
# on them. `quality` is the CI command and is made of several of these.
NETWORK_TASKS = frozenset({"deny", "audit", "quality", "bun-install", "backlog", "skills"})
OFFLINE_PAIRS = {"deny": "deny-offline", "audit": "audit-offline"}


@pytest.fixture(scope="module")
def tasks() -> dict:
    return tomllib.loads(PIXI.read_text(encoding="utf-8"))["tasks"]


def dependencies(task: dict | str) -> list[str]:
    """The tasks one entry depends on; a plain string entry depends on nothing."""
    return list(task.get("depends-on", [])) if isinstance(task, dict) else []


def test_test_local_is_made_only_of_offline_safe_tasks(tasks):
    offending = sorted(set(dependencies(tasks["test-local"])) & NETWORK_TASKS)

    assert not offending, (
        "test-local runs after a restore, with --no-install and no network; "
        f"{offending} would fetch instead of checking"
    )


def test_test_local_uses_the_offline_advisory_tasks(tasks):
    """The offline wrappers exist for exactly this; using the fetching ones defeats them."""
    depends = dependencies(tasks["test-local"])

    assert "deny-offline" in depends
    assert "audit-offline" not in depends, (
        "an offline audit steps aside with a message; it belongs on pre-push, not in the gate"
    )


def test_test_local_includes_the_documentation_and_doctests(tasks):
    """A guide and a doctest can both break a change; the gate has to see them."""
    depends = dependencies(tasks["test-local"])

    assert "check-docs" in depends
    assert "test-doc" in depends


def test_quality_keeps_the_fetching_checks(tasks):
    """CI has the network, so the authoritative run still audits and fetches."""
    depends = dependencies(tasks["quality"])

    assert "deny" in depends
    assert "audit" in depends


def test_every_offline_wrapper_exists_for_its_fetching_counterpart(tasks):
    for fetching, offline in OFFLINE_PAIRS.items():
        assert fetching in tasks
        assert offline in tasks, f"{fetching} needs an offline counterpart for the pre-push hook"
