---
id: CLI-69
title: Implement clifacet commands macro for impl blocks with self context
status: To Do
assignee: []
created_date: '2026-11-19 14:00'
labels: []
milestone: m-18
dependencies:
  - CLI-67
  - CLI-68
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the `#[clifacet::commands]` procedural macro attribute on `impl` blocks (decision-24). The macro transforms struct methods into executable CLI commands with parameter-to-argument mapping and doc-comment help extraction. Commands receive `&self`, providing zero-overhead, direct access to global CLI options and `self.cx` without parameter drilling. Support flexible return types via an `IntoOutcome` trait, accepting `()`, `Result<(), E>`, and serializable output types.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 #[clifacet::commands] on impl blocks maps methods to CLI commands with argument and help extraction
- [ ] #2 Command methods access &self, global flags, and initialized self.cx directly without parameter drilling
- [ ] #3 Return types implement IntoOutcome, converting (), Result<(), E>, and typed outputs into runner outcomes
- [ ] #4 Behavior tests verify end-to-end command dispatch, flag passing, and context access across commands
<!-- AC:END -->
