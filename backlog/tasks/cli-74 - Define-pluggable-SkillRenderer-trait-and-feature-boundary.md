---
id: CLI-74
title: Define pluggable SkillRenderer trait and feature boundary
status: Done
assignee: []
created_date: '2026-11-19 14:30'
labels: []
milestone: m-20
dependencies:
  - CLI-73
ordinal: 74000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Define the zero-dependency `clifacet::skill::SkillRenderer` trait behind the `skill` Cargo feature (decision-25). The trait receives `CliMetadata` and returns formatted skill documentation. Provide traits and macro extension hooks in `#[clifacet::app]` allowing developers to plug in custom rendering implementations (custom string builders, Tera, Askama) without requiring templating dependencies.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 skill Cargo feature and clifacet::skill::SkillRenderer trait are defined with zero third-party dependencies
- [x] #2 #[clifacet::app] accepts custom renderer types via skill attribute
- [x] #3 Unit tests verify that a custom user-defined SkillRenderer produces expected output
- [x] #4 Regression tests verify zero extra dependencies when skill feature is disabled
<!-- AC:END -->
