---
id: CLI-16
title: 'Qualify feature, operating system and MSRV matrix'
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-5
dependencies:
  - CLI-12
  - CLI-14
  - CLI-15
type: task
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Prove rather than assume compatibility.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CI covers no-default, default, interactive, TUI and all-features as supported.
- [ ] #2 Supported Linux, macOS and Windows stable jobs pass with headless tests where relevant.
- [ ] #3 Declared MSRV and supported feature/dependency combinations are checked and documented.
<!-- AC:END -->
