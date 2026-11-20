---
id: CLI-78
title: Context aware dynamic shell completions
status: Done
assignee: []
created_date: '2026-11-19 14:30'
labels: []
milestone: m-20
dependencies:
  - CLI-67
ordinal: 78000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement context-aware dynamic shell completions behind the `completions` Cargo feature (decision-25). Provide `mycli completions <shell>` generating completion scripts for Bash, Zsh, Fish, and PowerShell. Support dynamic argument completion callbacks annotated with `#[clifacet(complete = ...)]`, granting completion providers access to `&self.cx` to query live context state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 completions Cargo feature provides mycli completions for Bash, Zsh, Fish, and PowerShell
- [x] #2 #[clifacet(complete = ...)] connects argument completion callbacks to live context state
- [x] #3 Integration tests verify static completion script generation across all supported shells
- [x] #4 Integration tests verify dynamic completion resolution using mock context state
<!-- AC:END -->
