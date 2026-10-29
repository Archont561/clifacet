---
id: decision-8
title: Plain repository documentation for 0.1.0
date: '2026-10-29 15:28'
status: accepted
---
## Context
The maintainer wants repository-root `docs/` and a concise README like qgis-sdk, not a separate documentation website for v0.1.0.

## Decision
Write user and contributor guides as plain files in root `docs/`; keep root README brief, with purpose, status, packages, a small real example, development entrypoints and links. Document features as delivered.

## Consequences
No documentation-site generator or hosted site is required for 0.1.0. Backlog `docs/` stores project planning and specifications and is not the user-facing root `docs/`. Validate snippets and links before release.
