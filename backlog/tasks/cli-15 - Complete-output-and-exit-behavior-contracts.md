---
id: CLI-15
title: Complete output and exit behavior contracts
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-4
dependencies:
  - CLI-13
type: task
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Finalize text/machine streams and app-owned exit mapping across modes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Machine-readable stdout contains only intended data and does not prompt or emit ANSI under the chosen policy.
- [ ] #2 Success, validation failure, cancellation and execution failure are distinct under a documented app policy.
- [ ] #3 BDD and parity tests cover representative outcomes.
<!-- AC:END -->
