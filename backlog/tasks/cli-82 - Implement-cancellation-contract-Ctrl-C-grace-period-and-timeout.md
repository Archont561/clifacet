---
id: CLI-82
title: Implement cancellation contract Ctrl-C grace period and timeout
status: Done
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies:
  - CLI-80
ordinal: 82000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the cooperative cancellation contract (decision-26). The first Ctrl-C triggers the `Cancel` token and allows a configurable grace period for graceful cleanup; a second Ctrl-C or grace period expiration hard-aborts the task. Map `Failure::cancelled` to exit code 130 in `ExitPolicy::standard()`. Add optional global `--timeout <duration>` flag yielding a clean execution failure when exceeded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 First Ctrl-C signal trips Cancel token and awaits configurable graceful cleanup period
- [x] #2 Second Ctrl-C or timeout expiration immediately aborts the active task
- [x] #3 Cooperative cancellation maps to exit code 130 and preserves caller-facing failure messages
- [x] #4 Optional --timeout global flag automatically aborts execution after the specified duration
<!-- AC:END -->
