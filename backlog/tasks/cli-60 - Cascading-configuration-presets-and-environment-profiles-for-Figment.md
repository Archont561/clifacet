---
id: CLI-60
title: Cascading configuration presets and environment profiles for Figment
status: Done
assignee: []
created_date: '2026-11-19 07:08'
updated_date: '2026-11-19 07:28'
labels: []
milestone: m-15
dependencies:
  - CLI-59
ordinal: 60000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement multi-source cascading configuration presets and profile switching for Figment (Decision-22 Phase 2, Milestone m-15). Applications can load cascading system, user, and local configuration files with one helper call, and switch profiles via environment variables.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 FigmentLoader provides a cascading preset loading system (/etc/<app>/config.toml), user (~/.config/<app>/config.toml), and local (.<app>.toml) files
- [x] #2 Profile selection is supported via environment variable (FIGMENT_PROFILE or <APP>_PROFILE) or explicit builder call
- [x] #3 Cascading sources merge in correct order: system < user < local < environment < CLI flags
- [x] #4 Missing optional files in the cascade are safely skipped while malformed files report actionable errors
- [x] #5 Integration tests prove cascading file merging and profile switching using Map environment
<!-- AC:END -->
