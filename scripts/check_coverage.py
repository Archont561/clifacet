#!/usr/bin/env python3
"""Enforce the Codecov source-line target against the generated LCOV report."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE_DIRS = (
    ROOT / "crates" / "clifacet" / "src",
    ROOT / "crates" / "clifacet-macros" / "src",
)


def coverage(report: str, source_dirs: tuple[Path, ...] = SOURCE_DIRS) -> tuple[int, int]:
    """Merge LCOV line hits for repository source files, and require both crates."""
    lines: dict[tuple[Path, int], int] = {}
    seen: set[Path] = set()
    current: Path | None = None
    containing: Path | None = None
    for raw in report.splitlines():
        if raw.startswith("SF:"):
            current = Path(raw[3:]).resolve()
            containing = next((directory for directory in source_dirs if current.is_relative_to(directory)), None)
            if containing is None:
                current = None
        elif raw.startswith("DA:") and current is not None:
            parts = raw[3:].split(",")
            if len(parts) < 2:
                raise ValueError(f"Malformed LCOV line: {raw}")
            line, hits = int(parts[0]), int(parts[1])
            if containing is not None:
                seen.add(containing)
            key = (current, line)
            lines[key] = max(lines.get(key, 0), hits)
        elif raw == "end_of_record":
            current = None
    missing = set(source_dirs) - seen
    if missing:
        raise ValueError("Missing source coverage for: " + ", ".join(sorted(str(p) for p in missing)))
    if not lines:
        raise ValueError("No source lines found in coverage report")
    return sum(hits > 0 for hits in lines.values()), len(lines)


def normalize_lcov(report: str) -> str:
    """Recompute LCOV summaries from line hits merged across macro executions."""
    result: list[str] = []
    record: list[str] = []
    for raw in report.splitlines():
        record.append(raw)
        if raw != "end_of_record":
            continue
        hits: dict[int, int] = {}
        for entry in record:
            if entry.startswith("DA:"):
                parts = entry[3:].split(",")
                if len(parts) < 2:
                    raise ValueError(f"Malformed LCOV line: {entry}")
                number, count = int(parts[0]), int(parts[1])
                hits[number] = max(hits.get(number, 0), count)
        for entry in record:
            if hits and entry.startswith("LF:"):
                result.append(f"LF:{len(hits)}")
            elif hits and entry.startswith("LH:"):
                result.append(f"LH:{sum(count > 0 for count in hits.values())}")
            else:
                result.append(entry)
        record.clear()
    if record:
        raise ValueError("Incomplete LCOV record: missing end_of_record")
    return "\n".join(result) + "\n"


def meets_target(covered: int, total: int) -> bool:
    """Compare exactly to 90% without rounding a near-miss up."""
    return total > 0 and 10 * covered >= 9 * total


def main() -> int:
    report = Path(sys.argv[1] if len(sys.argv) > 1 else "coverage/lcov.info")
    try:
        normalized = normalize_lcov(report.read_text(encoding="utf-8"))
        covered, total = coverage(normalized)
        report.write_text(normalized, encoding="utf-8")
    except (OSError, ValueError) as error:
        print(f"Coverage check failed: {error}", file=sys.stderr)
        return 1
    percent = 100 * covered / total
    print(f"Source line coverage: {covered}/{total} ({percent:.2f}%); target: 90.00%")
    return 0 if meets_target(covered, total) else 1


if __name__ == "__main__":
    sys.exit(main())
