---
id: CLI-86
title: Dispatch MCP tool calls on AsyncHandler with cancellation and progress
status: To Do
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies:
  - CLI-80
  - CLI-77
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Amend the MCP server implementation (CLI-77) to await `AsyncHandler` commands directly and offload synchronous handlers to `spawn_blocking` (decision-26). Connect MCP `notifications/cancelled` to per-call `Cancel` tokens, and map `ProgressSink` updates directly to MCP `notifications/progress`. Support concurrent tool calls sharing `Arc<Cx>`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MCP tools/call directly awaits AsyncHandler commands without nested blocking
- [ ] #2 Synchronous handlers execute safely via tokio::task::spawn_blocking
- [ ] #3 MCP cancellation notifications trigger the associated call's Cancel token
- [ ] #4 Progress sink events stream as MCP notifications/progress back to the agent client
<!-- AC:END -->
