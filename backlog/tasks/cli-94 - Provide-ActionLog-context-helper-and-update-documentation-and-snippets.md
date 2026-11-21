---
id: CLI-94
title: Provide ActionLog context helper and update documentation and snippets
status: Done
assignee: []
created_date: '2026-11-21 08:08'
updated_date: '2026-11-21 08:30'
labels: []
milestone: m-23
dependencies: []
type: enhancement
ordinal: 94000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Provide an ergonomic ActionLog helper in clifacet for logging and dry-run action dispatching to stderr, and update documentation and examples to remove direct clap dependency requirements and demonstrate dry-run controller authoring.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ActionLog helper is available in clifacet and supports dry-run aware logging to stderr
- [x] #2 Documentation removes direct clap dependencies from quickstarts and guides
- [x] #3 A guide section explains dry-run authoring and custom logging across streams in declarative controllers
- [x] #4 All snippets and docs pass check-docs and cargo tests
<!-- AC:END -->
