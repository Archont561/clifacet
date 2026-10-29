---
name: tdd
description: Develop a feature or fix using small test-first vertical slices and agreed public seams.
---

# Test-driven development

Identify the observable behavior and agree on consequential public seams. Write
one focused failing test, establish why it fails, make the smallest change
that passes, and then review/refactor without changing behavior. Prefer rstest
fixtures/cases, add properties only for real invariants, and retain failing
regression seeds. Run checks through Pixi. Do not turn an unrun red test into a
claim that it failed as intended.
