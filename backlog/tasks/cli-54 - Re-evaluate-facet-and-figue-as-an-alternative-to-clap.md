---
id: CLI-54
title: Re-evaluate facet and figue as an alternative to clap
status: To Do
assignee: []
created_date: '2026-11-08 02:00'
labels: []
dependencies: []
type: task
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 7 keeps clap (decision-2) and defers facet. figue 4.0.5 (facet 0.46, Rust 1.90) layers CLI, environment and config files with provenance, a `MockEnv` for tests, completions and a reflected model a form could walk — but its core is at least 58k lines against clap's ~20k, facet has published 118 versions in 19 months with one publisher, 0.50 has been a release candidate since June needing Rust 1.92, and clifacet's public API is built on clap types. Nothing could be compiled in the review because cargo there cannot reach crates.io. Not scheduled: it starts when its trigger holds.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Started only once facet 0.50 or later is stable and its rust-version is at or below clifacet's; the notes record the versions that met the trigger
- [ ] #2 The same small CLI is built with clap and with figue in CI, and clean dev, release and check CPU time, binary size and crate count are recorded for both
- [ ] #3 The notes say whether figue can parse one field's value from a string (prompt parity needs it) and whether its model reports where each value came from
- [ ] #4 The outcome is a proposed decision, and clifacet itself does not change in this task
<!-- AC:END -->
