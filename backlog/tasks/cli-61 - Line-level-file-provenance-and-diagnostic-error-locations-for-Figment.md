---
id: CLI-61
title: Line-level file provenance and diagnostic error locations for Figment
status: Done
assignee: []
created_date: '2026-11-19 07:08'
updated_date: '2026-11-19 07:28'
labels: []
milestone: m-15
dependencies:
  - CLI-60
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enhance Figment error diagnostics and provenance reporting so that dry-run provenance and configuration error reports clearly cite the exact file and setting path (Decision-22 Phase 2, Milestone m-15).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 FigmentLoader retains file provenance for every configuration entry extracted from Figment providers
- [x] #2 Command dry-run provenance lists Source::File with file path for values supplied by Figment files
- [x] #3 Diagnostics for invalid configuration values cite the originating file path and setting key
- [x] #4 Integration tests verify provenance display and diagnostic error citation
<!-- AC:END -->
