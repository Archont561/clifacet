"""The source-only LCOV coverage gate."""

from pathlib import Path

import pytest

from scripts.check_coverage import SOURCE_DIRS, coverage, main, meets_target, normalize_lcov

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


def test_the_report_path_is_taken_as_an_argument(tmp_path, capsys):
    """`main(argv)` rather than `sys.argv`: under doit, `sys.argv` is doit's own line.

    This is not hypothetical. `pixi run automation coverage` passes the report as an
    argument, and a `main()` that read `sys.argv` treated doit's first argument — the task
    name — as the report path, so the task failed on a file that never existed.
    """
    report = tmp_path / "lcov.info"
    # `main` uses the repository's own source directories, so the report has to name a
    # file in each of the two crates: a report that covers one is reported as missing.
    crates = [path for path in SOURCE_DIRS if path.is_dir()]
    records = [row for path in crates for row in (f"SF:{path / 'lib.rs'}", "DA:1,1", "end_of_record")]
    report.write_text(lcov(*records), encoding="utf-8")

    assert main([str(report)]) == 0
    assert "100.00%" in capsys.readouterr().out


def test_a_meaningless_argument_is_reported_not_ignored(tmp_path, capsys):
    """The old failure mode: a path that is not there must say so, not pass."""
    assert main([str(tmp_path / "absent.info")]) == 1
    assert "Coverage check failed" in capsys.readouterr().err
