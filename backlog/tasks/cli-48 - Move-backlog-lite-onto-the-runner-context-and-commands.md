---
id: CLI-48
title: 'Move backlog-lite onto the runner, context and commands!'
status: In Progress
assignee: []
created_date: '2026-11-08 01:59'
updated_date: '2026-11-08 15:11'
labels: []
milestone: m-12
dependencies:
  - CLI-46
  - CLI-47
  - CLI-33
type: task
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The proof that decision-19's first sprint pays for itself, in the repository's only tree. Estimated in the review: app.rs (120 lines) goes, main.rs (17 lines) becomes one call, commands/mod.rs (125 lines) becomes about ten, invocation.rs (134 lines) becomes about sixty as a helper module, and the places touched to add a command go from seven to one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 main.rs is one call to the runner, app.rs is gone, and commands/mod.rs is a `clifacet::commands!(..)` registration
- [x] #2 The program backlog-lite spawns is part of its context, so the CLIFACET_BACKLOG_BIN variable is gone and the tests inject a fake
- [x] #3 The thread_local! run records in crates/clifacet/tests/command_tree.rs and the library BDD bindings are replaced by a context
- [x] #4 Every existing example test and scenario passes; bindings change only how they wire the application, never what they assert
- [x] #5 The notes record the line counts before and after and the number of places touched to add a command
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## What moved (AC#5 measurements)

Lines in `examples/backlog-lite/src` at `d1724a3f` → now:

| File | Before | After | What happened |
| --- | --- | --- | --- |
| `main.rs` | 17 | **9** | one call: `clifacet_backlog_lite::run_app()` |
| `app.rs` | 141 | **0, deleted** | the pipeline was clifacet's; the command moved into `lib.rs` |
| `invocation.rs` | 135 | 83 | now pure: `invocation_of(request, &Program)`, no spawning, no env |
| `program.rs` | — | 83 | **new**: the `Program` context and the single place that spawns |
| `lib.rs` | 51 | 125 | the `PlannedWith<Program>` impl, the `app`/context wiring, the re-exports |
| `commands/mod.rs` | 77 | 90 | +`From<Input> for Request` (what a bare `bl` means) and its docs |
| 7 leaf files + `status.rs` | 375 | 375 | untouched |
| **src total** | **857** | **826** | |

## The places touched to add a command did *not* change

The task estimated "seven places to one". Recording honestly: that reduction was already delivered by **CLI-33**, in the previous PR — the review's seven were `mod`, the `forward` re-export, the `Verb` variant, the `Action` variant, the `pub use`, the `request_of` arm and the `invocation_of` arm. CLI-48 moved backlog-lite *onto* that, so it inherits three places, and does not reduce them further:

1. the leaf's own file `src/commands/<leaf>.rs`;
2. one line in the `clifacet::commands!(..)` list (plus its `forward` re-export beside it);
3. one arm in `invocation_of`, which the compiler demands because the generated `Request` gains a variant.

Collapsing 2 and 3 is a real design question (a leaf could publish its own forwarding) and is not in this task's acceptance criteria, so it is left for a card rather than done here.

## TDD / BDD / clean-code evidence

**Red first.** `tests/program.rs` was written before any of this existed and failed to compile for exactly the intended reasons: `Program`, the context and `run_app` were absent (`E0432`/`E0599`/`E0061`). The `thread_local!` replacement started the same way: `a_run_is_recorded_in_the_context_it_was_given_and_nowhere_else` failed with "cannot find type `Ran`" before `Ran` existed.

**BDD.** The existing `features/plan.feature` and `features/questions.feature` scenarios are unchanged in what they assert — only the binding's wiring moved from `app(cli, …)` to `app(…).run_parsed(cli, cli.globals, …)`, which is AC#4's "bindings change only how they wire the application". No new scenario was written because AC#2's promise (the program comes from the context) is a dependency-injection statement with no user-visible workflow; it is proven in `tests/program.rs` instead, deliberately stated rather than silently skipped.

**Clean code.** `invocation.rs` became pure — it takes the program as an argument instead of reading the environment — and `program.rs` owns the one `Process` in the crate. The harness's constraint that `Handler::Input` is the root's `Input` was met with `From<Input> for Request`, which puts "a bare `bl` means list" in the declaration where it belongs rather than in a wrapper function.

**Verification:** `cargo test --workspace --all-features` 662 passed / 0 failed; `--no-default-features` green; `clippy --all-targets` clean in both feature sets; `cargo fmt --check` clean; `python scripts/check_docs.py` — 10 pages, every link resolves. The built binary was run by hand: `bl --dry-run done CLI-20 --yes` prints `move CLI-20 to Done` and the runnable line; an unanswerable confirmation still exits 2 naming `--yes`.
<!-- SECTION:NOTES:END -->
