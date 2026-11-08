---
id: CLI-47
title: Give commands the application's context
status: In Progress
assignee: []
created_date: '2026-11-08 01:59'
updated_date: '2026-11-08 13:14'
labels: []
milestone: m-12
dependencies:
  - CLI-46
type: task
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 2. Inside a tree, `self` is the parsed clap struct, so a command has nowhere to receive a store, a client or a clock: backlog-lite reads a global CLIFACET_BACKLOG_BIN variable and the tree tests record runs in a thread_local!. A context is passed by shared reference beside the input; `Command` itself does not change. The coherence of the blanket impl across crates and the E0119 for a type implementing both traits were compiled during the review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `Handler<Cx>` with `handle(&self, input, cx: &Cx)` is implemented for every `Command` by a blanket impl, so no existing implementation changes, proven by a test crate outside clifacet; implementing both traits for one type is a documented compile error
- [x] #2 A planned twin lets a command plan with the context, and the plan's work may borrow from it
- [x] #3 `#[clifacet::subcommands(.., context = T)]` threads `&T` to the chosen leaf, and a leaf without context still runs inside such a tree
- [x] #4 The runner builds the context after validation, on first use, and never for `--help`, `--version`, a usage error or a declined question; a context that cannot be built is an execution failure and nothing runs (scenarios)
- [x] #5 A question can read the context, for example to offer existing records as choices
- [x] #6 The authoring guide explains context versus input — dependencies versus what the user supplied — with a compiled snippet
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design agreed with the maintainer (ask_user, before any production code):

1. **Handler.** `clifacet::Handler<Cx: ?Sized>` mirrors `Command`: `handle(&self, input, cx: &Cx)` and a provided `preview<'a>(&'a self, input, cx: &'a Cx) -> Plan<'a, _>` that refuses. `impl<C: Command, Cx: ?Sized> Handler<Cx> for C` makes every command a handler under any context. Spike: an application crate's `impl Handler<Store> for Add` coexists with the blanket.
2. **Planned twin.** A second blanket from a twin into `Handler` is E0119 (spike), so the twin is `command::PlannedWith<Cx>` (`plan<'a>(&'a self, input, cx: &'a Cx) -> Plan<'a, _>`), and `#[clifacet::planned]` on its impl also emits the `Handler` impl (handle = plan then execute, preview = plan): one execution path, as `Planned` guarantees.
3. **Building the context.** `App::context(|| -> Result<Cx, E: Display>)`; any error becomes `Failure::execution("cannot prepare the application: {e}")`. Without it the context is `()`, so existing applications are unchanged.
4. **Questions.** The `Session` carries the runner's lazy context, type-erased: `session.context::<Cx>()` builds it on first use and returns a shared handle, so `Collect` is unchanged. `Cx: 'static`.

TDD, red first (feature work, so tdd → bdd → clean-code):

5. **Red.** `tests/context.rs` (AC 1, 2) written before any production code: E0432 for `clifacet::Handler` and `command::PlannedWith`, E0433 for `clifacet::planned`. Then the runner and session tests appended to the same file: E0599 for `Session::asks_the_user`/`context` and `App::context`. Green after `Handler` + blanket impl + `PlannedWith` + `#[clifacet::planned]` (5 passed) and after `Lazy`, `Session::context`/`with_context`/`sharing` and `App::context` (15 passed).
6. **AC 3, red first.** The `shelf` tree in `tests/context.rs` failed with "`context` is not a key of `#[clifacet::subcommands(..)]`; supported keys: `input = Type`, `output = Type`, `planned`". Green after the `context` key and the `Handler` generator: 19 passed. Macro-level tests in `clifacet-macros/src/tests.rs` cover the generated paths, the two rejections (`context` without `output`; `planned` with `context`) and the duplicate/malformed key; the trybuild fixture `tree_unknown_key.stderr` was re-blessed for the longer key list.
7. **BDD (AC 4, 5).** `features/context.feature` with seven scenarios, bound in `tests/context_bdd.rs`: 7 passed.
8. **AC 6.** A "What a command depends on" section in `docs/getting-started.md` — dependencies versus what the user supplied — with two compiled snippets (`scripts/check_docs.py --write`; 23 documented snippets pass) and the `.context(..)` row in the runner's table.

Outcome:

- `cargo test --workspace --all-features` and `--no-default-features`: green.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- `python scripts/check_docs.py`: 10 pages, every link resolves, every snippet is compiled, every documented name exists.

Scope notes:

- A planned twin cannot get a blanket `Handler` impl (E0119 overlapping the `Command` blanket), so `#[clifacet::planned]` writes the bridge beside the impl it belongs to; the task description anticipated this.
- `Session::asks_the_user` was not needed: `Session::plan()` already exposes `Plan::asks_the_user()`, so a question asks the plan it is running under.
- `planned` together with `context` in one tree is rejected: with a context the tree already previews through each leaf's `Handler::preview`, so `planned` would be a second, contradictory path.
- `#[clifacet::subcommands(context = ..)]` on the enum is the library-level seam; threading it through `commands!(context = T)` is CLI-33 (decision-19 item 3).
<!-- SECTION:NOTES:END -->
