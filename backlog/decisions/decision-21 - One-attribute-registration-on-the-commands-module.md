---
id: decision-21
title: One attribute registers a command folder
date: '2026-11-03 12:00'
status: accepted
---
## Context

decisions 15, 19 and CLI-50 settled how a folder of commands is registered, and left two
spellings of it:

- **a list** — `clifacet::commands!(list, show)` in `src/commands/mod.rs`, which writes the
  `mod` lines, the tree, the request and the root;
- **discovery** — `clifacet::commands!()` with no list in the same file, which reads the
  build script's report and writes the same things from it.

Both are a call *inside* the folder, and both leave the folder carrying registration: a
`mod.rs` whose first section is machinery, three lines of it for a folder of three commands.
The user-facing goal is a smaller surface than that — three files, and in the third one line
that says "this folder is the CLI".

A `commands!` line cannot simply move up to `src/lib.rs`. It is a declarative macro, so its
`mod x;` resolves at the invocation site, and the invocation site would then be `lib.rs`,
where `x` is not. That is the constraint that produced decision-15's "the call lives in the
folder's `mod.rs`" in the first place.

## Measured

Compiled in this repository while the change was made, not on paper:

1. **A proc-macro attribute and a function-like macro cannot share a name.** `#[clifacet::
   commands]` and `clifacet::commands!` in one crate is an error, whatever their arities, so
   the attribute needed a name of its own. `registry` was chosen: it names what it does — the
   registration — rather than the folder it does it to.
2. **An attribute cannot name a directory.** `#[path = "src/commands"] mod commands;` is
   refused, because `#[path]` names a file. There is no spelling of "this module is a folder
   whose contents come from over there".
3. **`include!` resolves nested `mod` lines against the included file's directory.** So a
   folder's `mod.rs` pulled in from `lib.rs` keeps working: its own `mod` lines still mean the
   files beside it. That is the single mechanism this decision rests on.
4. **A `mod` the macro writes still resolves against the invoking file**, included content or
   not. So the leaves and helpers the registry declares carry `#[path = "…/list.rs"]`
   explicitly, and the compiled file is still the one Rust reads — an edit inside the folder
   rebuilds the crate with no build-script regeneration.
5. **`include!` cannot carry inner attributes or `//!` docs.** A `mod.rs` almost always begins
   with them, and Rust's own message names the fragment rather than the attribute.
6. **A helper declared by the registry is private to the folder.** `mod _shared;` with no
   re-export is `E0603` from outside, which is exactly what CLI-50 promised, and the message
   now points at the attribute rather than at a `mod` line in the folder.

7. **An outlined module in an attribute's input was unstable until Rust 1.99.** CI's MSRV
   job compiled `#[clifacet::registry] mod commands;` with 1.88 and reported

   ```
   error[E0658]: non-inline modules in proc macro input are unstable
    --> examples/discovered/src/lib.rs:26:1
     |
   26 | pub mod commands;
     | ^^^^^^^^^^^^^^^^^
   ```

   The fix landed as a partial stabilization of `proc_macro_hygiene`
   ([rust-lang/rust#157857](https://github.com/rust-lang/rust/pull/157857), in 1.99.0):
   attribute and derive macros now receive an outlined module in its unloaded form. The
   workspace's floor is 1.88 and its pinned compiler is 1.99.1, which is exactly why every
   other job was green and only the MSRV job caught it — the ordinary case for a feature the
   pinned toolchain accepts.

## Decision

1. **`#[clifacet::registry(..)]` on `mod commands {}` is the registration.** One line in
   `src/lib.rs` replaces the call inside the folder, and it needs no list: the folder is the
   list, read from the build script's report as discovery already does.
2. **The folder's `mod.rs` is `include!`d**, so everything that is not registration stays
   where it was written — the folder's own `Request`, its `impl`s, its re-exports, its
   documentation in `//` comments. The macro emits the registration, then the `include!`.
   **The module is written with empty braces** — `mod commands {}` — because inline is the
   only spelling every supported compiler accepts. The braces carry no items; an empty body
   means "this module is the folder's file", and a body written there is the folder itself,
   kept as written beside the registration.
3. **This supersedes decision-15's requirement that registration be a call in the folder.**
   Its reasoning about `mod` resolution stands and is why `#[path]` is written per leaf; the
   conclusion that the call therefore has to live in the folder does not, because `include!`
   gives the folder its own resolution context again.
4. **The keys are the folder's, not a repetition of the tree's.** `output`, `globals`,
   `planned`, `version_off`, `name`, `about`, `long_about`, `root`, plus `path` for a folder
   whose name does not say where it is. There is deliberately **no `context`**: a command that
   needs the application's context declares `cx: &Cx` itself (CLI-52), so this line has
   nothing to repeat. There is **no module list** — the folder is the list, and a name written
   here is rejected with the message saying so.
5. **The MSRV is not raised for this.** 1.88 stays, because a registration syntax is not worth
   an eleven-release floor jump in a published 0.4 library, and `mod commands {}` costs one
   pair of braces and a sentence of explanation.

## Consequences

- **The registration can be read in one line.** `src/lib.rs` says the folder is the CLI, and
  `src/commands/` holds the commands and the folder's own types, with no machinery in it.
- **`mod commands {}` looks like an empty module and is not one.** That is the whole cost of
  keeping 1.88, so every place it is documented says so in a sentence: the attribute's own
  documentation, `docs/derive.md`, and both example crates. If clifacet's floor ever reaches
  1.99, the braces can be dropped with no other change — the macro and its tests treat the
  two forms alike, and `mod commands;` then works unchanged.
- **A `mod.rs` header is prose, not documentation.** `//!` is refused by `include!`; the
  folder documents itself in `//` comments and its documentation goes on the attribute's
  module. Enforced by a check that reads only the file's header, with a golden message.
- **The module is the folder and is named `commands`.** The attribute writes into the module
  it is given, so a module under another name has nowhere to put the registration; that is a
  golden compile-fail case, and the message names `clifacet::commands!(..)` as the door for
  anyone who wants a different layout.
- **Helpers stay private by construction.** The registry declares them beside the leaves and
  never re-exports them; `mod.rs` decides what leaves the folder. `/tmp` experiments during
  development and `examples/discovered/tests/ui/helper_from_outside.stderr` both hold this.
- **One diagnostic is worse than it was, and is accepted.** A helper's `mod` line is written
  by the macro, so the "module defined here" note of `E0603` points at the attribute in
  `src/lib.rs` instead of at a `mod _shared;` line in `src/commands/mod.rs` — the file that
  used to hold it has no such line any more. A proc macro cannot place a span on another
  file's line without a file handle, so faking it would be a lie; the error itself still
  points at the offending `use` in the caller, unchanged.
- **Discovery is still the build script's job.** `clifacet::build::commands()` is required,
  exactly as for `commands!()` with no list, and a registry without a report says so in one
  message that names the build script and the feature to enable.
- **`commands!(..)` stays.** It is the spelling for a folder whose members are not one per
  file, and the same tree is reachable from either; `examples/discovered` and
  `examples/backlog-lite` are the two folders built with the attribute.
