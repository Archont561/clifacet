"""Tests for reference documentation generators."""

from __future__ import annotations

from pathlib import Path
from tools._lib.gen_docs import (
    generate_features_mdx,
    generate_decisions_mdx,
    generate_exit_codes_mdx,
)


def test_generate_features_mdx_contains_all_features():
    content = generate_features_mdx()
    assert "# Feature Matrix" in content
    assert "`interactive`" in content
    assert "`tui`" in content
    assert "`serde`" in content
    assert "`schema`" in content
    assert "`async`" in content
    assert "`completions`" in content


def test_generate_decisions_mdx_contains_adrs():
    content = generate_decisions_mdx()
    assert "# Architectural Decisions Index" in content
    assert "decision-1" in content
    assert "decision-26" in content


def test_generate_exit_codes_mdx_contains_contract():
    content = generate_exit_codes_mdx()
    assert "Standard Exit Codes" in content
    assert "FailureKind::Execution" in content
    assert "FailureKind::Validation" in content
    assert "FailureKind::Cancelled" in content
