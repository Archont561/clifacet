---
id: CLI-41
title: Lay out both examples one file per command
status: In Progress
assignee: []
created_date: '2026-11-07 21:46'
labels: []
milestone: m-7
dependencies: []
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested on 2026-11-07: remove examples/simple/examples and give examples/simple and examples/backlog-lite a nested, file-per-command layout instead of one large lib.rs. This applies decision-15's recommended convention (a file per leaf, registration by hand in src/commands/mod.rs) without the deferred clifacet::commands! macro (CLI-33). Behaviour-preserving refactor: no public path, test or output changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 examples/simple/examples is removed, and every doc that pointed at its two demos points at the tests that drive the same published API
- [ ] #2 Each example keeps one file per command under src/commands/, registered by hand in src/commands/mod.rs (for backlog-lite, also the tree and its Request); lib.rs holds only the crate docs, the module list and re-exports
- [ ] #3 No test file changes and the same 185 example tests pass; every previously public item is still reachable at the crate root
- [ ] #4 Each example README's layout section describes the new structure
- [ ] #5 The task records the refactor exception: no behaviour changes, so no new BDD scenario
<!-- AC:END -->
