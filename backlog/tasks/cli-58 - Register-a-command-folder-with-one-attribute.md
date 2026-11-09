---
id: CLI-58
title: Register a command folder with one attribute
status: Done
assignee: []
created_date: '2026-11-03 11:00'
updated_date: '2026-11-03 12:00'
labels: []
milestone: m-13
dependencies: []
ordinal: 58000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A folder of commands is registered by a call inside it, so its `mod.rs` opens with machinery and `src/lib.rs` says nothing about the CLI. `#[clifacet::registry(..)]` moves the registration to the module declaration — one line where the application's own code is — and pulls the folder's `mod.rs` in beside the generated tree, so the folder holds only its commands and its own types. Supersedes decision-15's "the call lives in the folder"; see decision-21.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `#[clifacet::registry(..)] mod commands {}` emits the folder's `mod` lines, `Commands`, the request, `Input` and `Cli`, from the report `clifacet::build::commands()` writes
- [x] #2 The folder's `mod.rs` is included, so a `Request`, an `impl` or a re-export written there means what it meant before; a `//!` header is refused with a message that says to use `//`
- [x] #3 A helper is declared and not re-exported, so reaching it from outside is `E0603`; the golden case is regenerated and the accepted diagnostic cost is recorded in decision-21
- [x] #4 The keys are the folder's — `output`, `globals`, `planned`, `version_off`, `name`, `about`, `long_about`, `root`, `path` — with no module list and no `context`; a module name is refused as a key
- [x] #5 Both `examples/discovered` and `examples/backlog-lite` (seven leaves and a `_task` helper) are converted, and the workspace's tests pass unchanged in count and outcome
- [x] #6 Unit tests cover the expansion, the request name, the key forwarding and each refusal; three `trybuild` cases cover the wrong module, a module list and an inner-doc header
- [x] #7 `pixi run lint`, `fmt-check`, `alint`, `check-docs` and `backlog-check` pass, and the changelog has the entry
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**The mechanism.** A proc-macro attribute cannot name a directory, and `#[clifacet::commands]` cannot exist because a proc-macro attribute and a function-like macro may not share a name — both measured, both in decision-21. What works is `include!`: a nested `mod x;` inside an included file resolves against *that file's* directory, so the folder keeps its own resolution context even though the registration runs from `src/lib.rs`.

**What stayed hard.** A `mod` the macro writes still resolves against the invoking file, so the registry writes `#[path = "…/list.rs"] mod list;` for every leaf and helper. That is not a hack around the include; it is the same absolute-path-per-leaf the decision-15 experiments measured, and it is what keeps the compiled file the one Rust reads.

**Four bugs found by building it.** The `"helper"` arm of the report decoder had been placed inside the `"command" | "group"` pattern whose inner match fell through to `helpers`, so helpers were decoded but never reached the declaration pass — found by writing the intermediate `Declared` to `/tmp` and comparing it against the report. `Declared.request` was a `"Request"` sentinel, so `request = Request` was indistinguishable from unset and a folder asking for `Request` got `Action`; it is `Option<Ident>` now, resolved at emit time. The request-name rule itself had to become "`Action` for a registry, `Request` for a list", because the registry's folder is exactly where a hand-written `Request` lives. And `declarations` needed no `super::` prefix once it was written from inside the folder, which is the duplicate-`task`-declaration error the enum surfaced.

**A helper is not a command.** `examples/backlog-lite`'s shared code was `task.rs`; a file without an `_` prefix is a command by CLI-50's convention, so it became a phantom `Action::Task` with an `Input` missing `json`. Renaming it `_task.rs` is the fix the convention asks for, not a special case in the scanner.

**The accepted cost, recorded rather than hidden.** A proc macro cannot put a span on another file's line, so the private helper's `mod` line carries the attribute's span and the `E0603` note points at `src/lib.rs`. Faking a span would be a lie; the error itself is unchanged. Decision-21 says so, and the regenerated golden shows it.

**The MSRV, found by CI and not by the sandbox.** `#[clifacet::registry] mod commands;` compiled
everywhere except the MSRV job, which reports `error[E0658]: non-inline modules in proc macro
input are unstable`: an outlined module in an attribute's input was stabilized only in Rust
1.99 ([rust-lang/rust#157857](https://github.com/rust-lang/rust/pull/157857)). The pinned
compiler here is 1.99.1, so nothing local could have caught it. The floor was kept and the
module is written `mod commands {}` instead — inline, accepted by every supported compiler,
with the empty body meaning "fill this from the folder's file". Recorded in decision-21
with the way back: once the floor reaches 1.99, the braces come off and nothing else changes.

**A second CI finding, and a better message.** The Windows job failed on the
`registry_inner_docs` golden, because the message named the folder's `mod.rs` by its absolute
path — fine on one machine, a different string on every other, and a per-platform golden file
waiting to happen. The macro now reports the file relative to the crate with `/` separators
(`src/commands/mod.rs`), which is both what a reader wants and what one golden can hold.

**Verification.** `cargo test --workspace` passes with the same count as before the feature; `fmt-check`, `alint`, `lint` (with `-D warnings`, expansions and both examples included), `check-docs`, `prose-check` and `backlog-check` pass. Both example folders were converted and their own test suites — discovery, privacy, reaching — are unchanged.
<!-- SECTION:NOTES:END -->
