---
id: CLI-2
title: Add Alint and tested Python repository checks
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-0
dependencies:
  - CLI-1
type: task
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Separate declarative repository policy from project-specific semantic checks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Alint checks required files and workflow policy with a reviewed baseline if needed.
- [x] #2 Python checks have pytest coverage and do not duplicate policy parsers owned by other tools.
- [x] #3 CI and local gates call the same documented Pixi entrypoints.
<!-- AC:END -->

## Notes

- Verified: alint 0.20.1 passes all 28 rules with a reviewed committed baseline, and CI and `scripts/tests` call the same `pixi run` tasks.
- #2 now runs under pytest (`pixi run --frozen test-scripts`). The suite was migrated from `unittest.TestCase` classes to pytest fixtures, `parametrize` and plain asserts, and `pytest` 9.2.0 was added to `[pypi-dependencies]` and `pixi.lock`. doc-5 records the runner.
