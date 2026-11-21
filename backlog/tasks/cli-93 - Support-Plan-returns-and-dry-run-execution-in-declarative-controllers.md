---
id: CLI-93
title: Support Plan returns and dry-run execution in declarative controllers
status: Done
assignee: []
created_date: '2026-11-21 08:08'
updated_date: '2026-11-21 08:25'
labels: []
milestone: m-23
dependencies: []
type: enhancement
ordinal: 93000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enhance declarative controllers (#[clifacet::commands]) to accept methods returning Result<Plan<'a, T>, Failure>. In dry-run mode, format and describe the plan; in real mode, execute the plan through the same code path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 IntoOutcome and FallbackFormatOutcome handle Result<Plan<'a, T>, Failure>
- [x] #2 In --dry-run mode, controller commands returning Plan output the plan description or structured JSON without executing side-effects
- [x] #3 In normal mode, controller commands returning Plan execute the plan and render the output
- [x] #4 Unit tests verify both dry-run and real execution for controller methods returning Plan
<!-- AC:END -->
