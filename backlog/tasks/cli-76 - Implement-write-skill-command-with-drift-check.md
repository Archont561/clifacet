---
id: CLI-76
title: Implement write-skill command with drift check
status: Done
assignee: []
created_date: '2026-11-19 14:30'
labels: []
milestone: m-20
dependencies:
  - CLI-74
  - CLI-75
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the built-in `write-skill` CLI command (decision-25). Support writing the generated skill document to standard output (`--stdout`), the default project skill location (`.agents/skills/<name>/SKILL.md`), or an explicit file path (`-o/--output`). Provide a `--check` flag that compares the committed file with the rendered output and exits with a non-zero code upon drift, enabling CI enforcement.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 mycli write-skill supports --stdout, -o/--output, and default .agents/skills/ location
- [x] #2 mycli write-skill --check compares on-disk file with current rendering and reports drift
- [x] #3 Integration tests verify file creation, stdout streaming, and --check exit codes on match and drift
- [x] #4 CI integration guide explains adding write-skill --check to repository quality gates
<!-- AC:END -->
