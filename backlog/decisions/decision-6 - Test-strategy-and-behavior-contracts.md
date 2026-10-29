---
id: decision-6
title: Test strategy and behavior contracts
date: '2026-10-29 15:28'
status: accepted
---
## Context
The maintainer requested rstest, rstest-bdd and proptest.

## Decision
Use rstest fixtures and case tables; rstest-bdd for meaningful user-visible workflows; proptest for general invariants; and trybuild for macro compile-pass/fail diagnostics. Run nextest and doctests separately. Test public seams and use scripted or headless adapters where appropriate; commit useful proptest regression seeds.

## Consequences
BDD examples are not a replacement for parameterized, property or component tests. Verify dependency compatibility and MSRV rather than assuming a version pairing. Cover redaction without printing canaries on failure.
