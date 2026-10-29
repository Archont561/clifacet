---
id: CLI-19
title: Review and remove the rstest-bdd advisory exception
status: To Do
assignee: []
created_date: '2026-10-29 15:46'
labels: []
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
