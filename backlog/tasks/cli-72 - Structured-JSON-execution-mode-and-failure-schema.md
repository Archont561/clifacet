---
id: CLI-72
title: Structured JSON execution mode and failure schema
status: Done
assignee: []
created_date: '2026-11-19 14:30'
updated_date: '2026-11-20 09:02'
labels: []
milestone: m-19
dependencies: []
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the global `--json` execution mode and standardized failure schema (decision-25). When `--json` is supplied, successful command executions serialize the returned data structure (`T: Serialize`) to stdout. When a command fails or validation rejects arguments, serialize the `Failure` into a machine-readable JSON object on stderr with proper exit codes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Global --json flag formats successful command outputs as structured JSON on stdout
- [x] #2 Command failures and validation errors format as standardized JSON error payloads on stderr
- [x] #3 Non-interactive and piped executions without a TTY preserve strict separation of data and logs
- [x] #4 Integration tests verify JSON output structure, exit codes, and deserializability across success and failure
<!-- AC:END -->
