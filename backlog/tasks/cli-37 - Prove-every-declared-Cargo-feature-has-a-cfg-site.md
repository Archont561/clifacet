---
id: CLI-37
title: Prove every declared Cargo feature has a cfg site
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
labels: []
milestone: m-9
dependencies: []
type: task
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A declared feature that no cfg mentions is a promise the crate does not keep. Extend the feature-boundary script so the matrix rows stay derived from the manifest and a feature without a cfg site fails, then retire or implement the json feature on that evidence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 The feature script fails when a declared feature has no `cfg(feature = "…")` site, and its own test covers that failure.
- [ ] #2 Matrix rows are still derived from the manifest, and `--print-matrix` keeps its shape.
- [ ] #3 A feature without a `cfg` site is not allowlisted silently: any exception carries a written reason in the script.
- [ ] #4 The guard runs in CI, in the same job as the existing feature-boundary check.
<!-- AC:END -->
