---
id: CLI-7
title: Implement companion derive and clap validation slice
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-1
dependencies:
  - CLI-6
type: task
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Prove the proposed clap-first derive against a representative existing-style clap command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A derived example resolves non-interactively and final validation uses its original clap Command or a documented typed bridge.
- [ ] #2 Aliases, defaults and a custom parser representative case are tested; unsupported cases fail explicitly.
- [ ] #3 trybuild covers valid and invalid derives; no private clap APIs are used.
<!-- AC:END -->
