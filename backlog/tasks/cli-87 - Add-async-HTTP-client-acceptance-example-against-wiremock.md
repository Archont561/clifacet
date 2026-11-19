---
id: CLI-87
title: Add async HTTP client acceptance example against wiremock
status: To Do
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies:
  - CLI-81
  - CLI-82
  - CLI-84
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create an end-to-end acceptance example crate `examples/http-lite` acting as the async counterpart to `examples/backlog-lite` (decision-26). Implement a CLI interacting with HTTP endpoints using reqwest or hyper, validated against a local wiremock server in automated integration tests. Prove cooperative cancellation, `--dry-run` inspection, and progress reporting in real scenarios.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 examples/http-lite crate demonstrates async controllers, reqwest integration, and shared context
- [ ] #2 Automated integration tests run commands against wiremock mock HTTP services
- [ ] #3 Cancellation, --dry-run, and progress reporting are tested in realistic client workflows
- [ ] #4 Example crate compiles cleanly and is included in workspace quality checks
<!-- AC:END -->
