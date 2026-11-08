---
id: CLI-56
title: Adopt Vale for the prose
status: In Progress
assignee: []
created_date: '2026-11-08 17:19'
labels: []
milestone: m-13
dependencies: []
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-20: adopt Vale for the prose and keep alint for the repository. vale.toml is authored; scripts/prose.py renders .vale.ini and .vale/styles/; prose-check runs in test-local, quality and the pre-commit hook.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Vale lints README.md, AGENTS.md and docs/**/*.md, with the Microsoft and write-good styles pinned to release URLs
- [ ] #2 vale.toml is the authored source and `prose-check` fails when .vale.ini or .vale/styles/ has drifted from it
- [ ] #3 Every inherited rule the repository sets its own way is declared in vale.toml with the reason, and a `NO` without one is refused
- [ ] #4 The repository's own style carries the American-English substitutions the prose needs, and the vocabulary the names
- [ ] #5 prose-check runs in test-local, quality and the pre-commit hook, and works offline
- [ ] #6 AGENTS.md says which of Vale and alint owns a finding, and docs/development.md explains how to change the check
- [ ] #7 The prose the check found inconsistent is corrected without touching a code span or an API name
<!-- AC:END -->
