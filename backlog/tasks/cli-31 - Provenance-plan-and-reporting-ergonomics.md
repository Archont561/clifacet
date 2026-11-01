---
id: CLI-31
title: 'Provenance, plan and reporting ergonomics'
status: To Do
assignee: []
created_date: '2026-11-01 22:34'
labels: []
milestone: m-6
dependencies: []
type: task
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Provenance derived from the parse result, a Plan value every application can print or execute, parity sugar for tests, and one failure renderer. Decision-18 items 4-7.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 `Provenance::from_matches` records `Source::Argv`, `Source::Default` and `Source::Environment` for every present value, and records nothing for an absent one.
- [ ] #2 A prompted value and a parsed value compose into one `Provenance`, and a secret is never recorded with a value.
- [ ] #3 `Command::plan` returns a `Plan` that can be described or executed, and `execute` is the only execution path so the dry run cannot drift.
- [ ] #4 A test asserts that a described plan and an executed plan agree, using the parity harness rather than a bespoke comparison.
- [ ] #5 `report::render` writes a `Failure` with its kind and message, and adds the usage text for a usage failure, on the stream `ExitPolicy` implies.
- [ ] #6 Parity construction gets a shorthand that a parameterised test can use without the three lines of `Outcome` plumbing.
<!-- AC:END -->
