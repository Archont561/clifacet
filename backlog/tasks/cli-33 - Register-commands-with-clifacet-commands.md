---
id: CLI-33
title: Register one command per file
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
updated_date: '2026-11-06 14:30'
labels: []
milestone: m-7
dependencies:
  - CLI-32
type: task
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Deferred** (decision-15, accepted in part on 2026-11-06): do not start until CLI-32 has moved `examples/backlog-lite` to the declarative tree and recorded what a `clifacet::commands!` macro would still save; if the saving is the `pub mod` lines and one enum, propose closing this card instead. If built: a command lives in its own module with a readable registration line, the registry is explicit, and `Request` stays hand-written (decision-14). Distributed registration, a proc-macro directory scan and a `clifacet-build` crate are refused. "A file per leaf" is a recommended convention in the guide, not a required layout. This card also carries CLI-34's macro-layering criteria (decision-16, narrowed), because it would ship the first exported `macro_rules!`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Before any code, the notes record CLI-32's before/after measurement and the maintainer's go-ahead; without both this card does not start.
- [ ] #2 A command lives in its own module and is registered by a line a reader can follow without reading a macro; the guide recommends one file per leaf without requiring it.
- [ ] #3 The registry is explicit: nothing is discovered globally and nothing is registered by initialising a static, and `Request` is written by the application, not generated.
- [ ] #4 Registering two commands with the same name fails to compile rather than resolving by order.
- [ ] #5 Dispatch stays compile-time, so no subcommand arrives as an untyped `OsString` that skipped validation.
- [ ] #6 The reference application registers its commands file by file, with its tests unchanged.
- [ ] #7 Decision-16's layering rule is written in the authoring guide and every public macro, attribute and derive states which layer it belongs to and what it expands to, so a reader can answer "which macro do I need?" without opening the source.
- [ ] #8 The exported macro uses `$crate` for every internal path, forwards arguments to the attribute as raw tokens or the attribute's `bridge`/`check` accept them as fragments, and crossing the boundary — a field attribute where a command is expected, or the reverse — fails to compile with a message that says what to write instead, covered by trybuild.
<!-- AC:END -->
