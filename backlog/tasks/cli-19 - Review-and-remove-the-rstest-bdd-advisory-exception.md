---
id: CLI-19
title: Review and remove the rstest-bdd advisory exception
status: To Do
assignee: []
created_date: '2026-10-29 15:46'
labels: []
milestone: m-4
dependencies: []
priority: medium
type: task
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Track the temporary RUSTSEC-2026-0374 exception in cargo-deny and cargo-audit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Review whether rstest-bdd or its transitive dependencies can avoid the unmaintained basic-toml crate.
- [ ] #2 Remove both ID-wide exceptions when possible; otherwise document the current dependency path and review outcome.
<!-- AC:END -->

## Notes

- 2026-10-30: assigned to m-4 (parity and safety), which carries the dependency and safety guarantees; the task had no milestone. Reassign if the maintainer prefers m-5. The exception itself is unchanged: `.cargo/audit.toml` still ignores RUSTSEC-2026-0374 until #1 is reviewed.
