---
id: doc-4
title: Testing and acceptance strategy
type: other
created_date: '2026-10-29 15:28'
updated_date: '2026-10-29 15:28'
---
# Test layers

- rstest: public-seam fixtures, policy tables, validation, errors and output.
- rstest-bdd: representative workflows in `tests/features/*.feature` with steps bound to public harnesses.
- proptest: determinism, supported non-secret canonical-argv round trips, redaction and documented mode equivalence. Preserve meaningful regression seeds.
- trybuild: derive acceptance and useful compile-failure diagnostics.
- Headless ratatui TestBackend and a scripted framework-owned prompt plan; do not assume inquire offers public backend injection.

# First behavior scenarios

1. Missing required value under non-interactive policy reports an error without constructing a prompter or executing.
2. Yes accepts confirmation only and does not invent missing required values.
3. Equivalent representative inputs through flags and prompt, then TUI when available, produce the same validated typed input.
4. Cancellation does not invoke the handler.
5. Framework-owned output redacts a secret canary without emitting the canary in a failing test diagnostic.

Dry-run scenarios belong to the deferred plan/execute feature. Run nextest and Cargo doctests separately. CI additionally checks supported features, OSes, MSRV and docs. Specific seams and fixture APIs must be agreed as implementation evolves.
