---
id: CLI-84
title: Implement frontend progress rendering across execution modes
status: Done
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies:
  - CLI-79
  - CLI-72
ordinal: 84000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement presentation-tier rendering for `ProgressSink` events across all supported frontends (decision-26). In plain interactive TTY mode, display a single-line progress indicator on stderr; in non-TTY or piped mode, remain completely silent; in TUI mode, render a Ratatui gauge; and when `--json` (CLI-72) is active, stream NDJSON progress events on stderr.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Plain interactive TTY mode displays ephemeral progress updates on stderr
- [x] #2 Non-TTY, redirected, or batch mode suppresses all progress rendering
- [x] #3 TUI frontend renders live progress via Ratatui gauge widgets
- [x] #4 --json mode emits structured NDJSON progress events to stderr without contaminating stdout data
<!-- AC:END -->
