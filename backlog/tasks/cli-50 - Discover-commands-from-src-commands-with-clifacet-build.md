---
id: CLI-50
title: 'Discover commands from src/commands with clifacet::build'
status: To Do
assignee: []
created_date: '2026-11-08 01:59'
labels: []
milestone: m-13
dependencies:
  - CLI-33
type: task
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 4, which reverses decision-15 item 2's refusal: discovery lives in clifacet behind a `build` feature, not in a third crate. The application's build script reports each folder's commands, groups and helpers; `clifacet::commands!()` with no list reads its own folder's report and emits `mod` declarations at the invocation site, so decision-15's objections — `#[path]` absolute paths and errors in OUT_DIR — do not arise, and no proc macro reads the directory (decision-15 item 4 stands). Measured in the review on the 2-core sandbox: clean dev +1%, clean release +7%, clean check +12% CPU (a fixed ~6 s), adding a command 0.8 s, a no-op rebuild 0.2 s.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `clifacet::build::commands()` behind a `build` feature, called from the application's build.rs, reports every folder's commands, groups and helpers through a versioned environment protocol and reruns when a folder changes; `commands_in(dir)` scans another folder
- [ ] #2 `clifacet::commands!()` with no list finds its own folder with `Span::local_file()` resolved against the compiler's working directory, and the msrv job proves it at Rust 1.88
- [ ] #3 `_name.rs` and `_name/` are private helper modules, visible to their folder and the groups under it and public only when `mod.rs` re-exports an item; a folder whose name starts with `_` is never a group
- [ ] #4 Every mistake is reported at the application's own file: a syntax error in a helper, a command file without its struct, a helper reached from outside its folder (E0603), each covered by a test
- [ ] #5 Dropping a file into src/commands adds a command with no other edit, and the build-time cost is re-measured and recorded
- [ ] #6 Moving between `commands!(list, ..)` and `commands!()` changes only that call, and no new crate is published
<!-- AC:END -->
