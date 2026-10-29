---
name: bdd
description: Describe meaningful user-visible workflows with executable Given/When/Then scenarios.
---

# Behavior-driven development

Write scenarios for public promises, mode policy, validation, cancellation,
output and safety, not internal implementation steps. Store `.feature` files
and step bindings with the relevant crate's tests; use `rstest-bdd`. Keep each
scenario independent and drive the same public seams used by consumers.
Use rstest tables, proptest and component tests for combinatorial cases or
implementation details instead of duplicating them in prose.
