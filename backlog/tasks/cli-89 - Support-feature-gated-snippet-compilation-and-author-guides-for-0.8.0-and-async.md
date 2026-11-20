---
id: CLI-89
title: >-
  Support feature-gated snippet compilation and author guides for 0.8.0 and
  async
status: Done
assignee: []
created_date: '2026-11-20 22:28'
updated_date: '2026-11-20 22:45'
labels: []
milestone: m-22
dependencies:
  - CLI-88
ordinal: 89000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enable check_docs.py and snippet test suite to compile feature-gated snippets, then author comprehensive guides for structured JSON output, shell completions, settings, and native async commands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 check_docs.py supports feature-gated code fences (rust,features=...)
- [x] #2 Snippet test harness compiles feature-gated snippets cleanly
- [x] #3 Document structured output (--json) and CLI schema introspection
- [x] #4 Document dynamic shell completions and settings configuration
- [x] #5 Document core async traits, cancellation, and progress (CLI-79) with unreleased callout
<!-- AC:END -->
