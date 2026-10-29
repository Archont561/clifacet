"""Tests for the source-only LCOV coverage gate."""

import unittest
from pathlib import Path

from scripts.check_coverage import coverage, meets_target, normalize_lcov

A = Path("/repository/crates/a/src")
B = Path("/repository/crates/b/src")


class CoverageTests(unittest.TestCase):
    def test_only_source_lines_count_and_duplicate_hits_are_merged(self):
        data = "\n".join([
            "SF:/repository/crates/a/src/lib.rs", "DA:1,0", "DA:2,1", "end_of_record",
            "SF:/repository/crates/a/src/lib.rs", "DA:1,2", "end_of_record",
            "SF:/repository/crates/b/src/lib.rs", "DA:3,0", "end_of_record",
            "SF:/repository/crates/b/tests/test.rs", "DA:9,0", "end_of_record",
        ])
        self.assertEqual(coverage(data, (A, B)), (2, 3))

    def test_missing_crate_fails_closed(self):
        data = "SF:/repository/crates/a/src/lib.rs\nDA:1,1\nend_of_record\n"
        with self.assertRaisesRegex(ValueError, "Missing source coverage"):
            coverage(data, (A, B))

    def test_no_lines_fails_closed(self):
        data = "SF:/repository/crates/a/src/lib.rs\nend_of_record\n"
        with self.assertRaisesRegex(ValueError, "Missing source coverage"):
            coverage(data, (A,))

    def test_normalize_reconciles_summary_with_merged_line_hits(self):
        data = "SF:/repository/crates/a/src/lib.rs\nDA:1,1\nDA:2,0\nDA:2,4\nLF:2\nLH:1\nend_of_record\n"
        normalized = normalize_lcov(data)
        self.assertIn("LF:2\nLH:2\n", normalized)
        self.assertIn("DA:2,4", normalized)

    def test_normalize_rejects_incomplete_record(self):
        with self.assertRaisesRegex(ValueError, "Incomplete LCOV"):
            normalize_lcov("SF:/repository/crates/a/src/lib.rs\nDA:1,1\n")

    def test_threshold_is_exact(self):
        self.assertTrue(meets_target(9, 10))
        self.assertFalse(meets_target(89, 100))
        self.assertFalse(meets_target(0, 0))

    def test_malformed_lines_fail(self):
        data = "SF:/repository/crates/a/src/lib.rs\nDA:broken\nend_of_record\n"
        with self.assertRaisesRegex(ValueError, "Malformed LCOV"):
            coverage(data, (A,))


if __name__ == "__main__":
    unittest.main()
