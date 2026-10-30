"""The source-only LCOV coverage gate."""

from pathlib import Path

import pytest

from scripts.check_coverage import coverage, meets_target, normalize_lcov

A = Path("/repository/crates/a/src")
B = Path("/repository/crates/b/src")


def lcov(*records):
    """Join LCOV records, one per line, as the gate reads them."""
    return "\n".join(records)


def test_only_source_lines_count_and_duplicate_hits_are_merged():
    data = lcov(
        "SF:/repository/crates/a/src/lib.rs",
        "DA:1,0",
        "DA:2,1",
        "end_of_record",
        "SF:/repository/crates/a/src/lib.rs",
        "DA:1,2",
        "end_of_record",
        "SF:/repository/crates/b/src/lib.rs",
        "DA:3,0",
        "end_of_record",
        "SF:/repository/crates/b/tests/test.rs",
        "DA:9,0",
        "end_of_record",
    )

    assert coverage(data, (A, B)) == (2, 3)


@pytest.mark.parametrize(
    ("data", "crates"),
    [
        pytest.param(lcov("SF:/repository/crates/a/src/lib.rs", "DA:1,1", "end_of_record"), (A, B), id="crate-absent"),
        pytest.param(lcov("SF:/repository/crates/a/src/lib.rs", "end_of_record"), (A,), id="no-lines"),
    ],
)
def test_missing_source_coverage_fails_closed(data, crates):
    with pytest.raises(ValueError, match="Missing source coverage"):
        coverage(data, crates)


def test_normalize_reconciles_summary_with_merged_line_hits():
    data = lcov(
        "SF:/repository/crates/a/src/lib.rs",
        "DA:1,1",
        "DA:2,0",
        "DA:2,4",
        "LF:2",
        "LH:1",
        "end_of_record",
    )

    normalized = normalize_lcov(data)

    assert "LF:2\nLH:2\n" in normalized
    assert "DA:2,4" in normalized


def test_normalize_rejects_incomplete_record():
    with pytest.raises(ValueError, match="Incomplete LCOV"):
        normalize_lcov(lcov("SF:/repository/crates/a/src/lib.rs", "DA:1,1"))


@pytest.mark.parametrize(
    ("hit", "total", "expected"),
    [
        pytest.param(9, 10, True, id="just-above-90-percent"),
        pytest.param(89, 100, False, id="just-below-90-percent"),
        pytest.param(0, 0, False, id="no-lines-is-not-success"),
    ],
)
def test_threshold_is_exact(hit, total, expected):
    assert meets_target(hit, total) is expected


def test_malformed_lines_fail():
    data = lcov("SF:/repository/crates/a/src/lib.rs", "DA:broken", "end_of_record")

    with pytest.raises(ValueError, match="Malformed LCOV"):
        coverage(data, (A,))
