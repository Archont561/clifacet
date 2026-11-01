---
id: decision-15
title: A file per command, with registration you can read
date: '2026-11-01 22:10'
status: proposed
---
## Context

The question this answers: should a clifacet application lay its commands out as files and
have them **discovered**, the way citty's `subCommands: { sub: () => import("./sub.mjs") }`
or a framework's file-based routing does?

In a compiled language there is no module reflection, so "autodiscovery" can only mean one
of three mechanisms. All three were considered, two were compiled:

1. **A build script** scans `src/commands/`, generates the tree, and the crate `include!`s it.
2. **A proc macro** reads the directory at expansion time instead.
3. **Distributed registration** (`inventory`, `linkme`, `ctor`): each command file registers
   itself into a link-time slice, and the CLI is assembled at run time.

Two constraints from accepted decisions bound the answer:

- **decision-1**: "Registration is explicit; derive macros provide metadata and adapters
  rather than hidden global registration." Mechanism 3 is the definition of hidden global
  registration, so adopting it means amending an accepted decision, not just adding a
  feature.
- **decision-14**: the tree is static and compiled in, because `clap` builds the whole
  command before parsing. citty's headline feature — lazy `import()` per subcommand — has
  no counterpart here.

## Measured

Everything below was compiled in this repository against `clap` 4.6.7 and then removed;
the working tree is unchanged.

**Mechanism 1 works.** A 45-line `build.rs` scanned `src/commands/*.rs`, sorted the file
names, and wrote an enum of `clap::Args` tuple variants; `src/commands/mod.rs` was one
`include!(concat!(env!("OUT_DIR"), "/discovered.rs"))`. Two tests passed: dropping a file
into the directory produced a parseable subcommand, and the generated variants appeared in
`render_help()`. Adding a file triggered a rebuild with no `build.rs` edit —
`cargo:warning=discovered ["extra", "greet", "list"]` — so directory tracking with
`rerun-if-changed` on the directory is reliable.

**But "a file is a command" was false on the first try.** The `pub mod` list in `mod.rs`
is a *second* registration list, so the new file alone did not compile:

```
error[E0433]: cannot find `extra` in `commands`
 --> /…/target/debug/build/clifacet-example-…/out/discovered.rs:3:28
```

Generating the `pub mod` lines does not fix it either: `mod` inside `include!`d content
resolves relative to the **included** file, not the includer —

```
error[E0583]: file not found for module `list`
  = help: to create the module `list`, create file
    "/…/target/debug/build/…/out/list.rs"
```

The only version that did work emitted `#[path = "/home/user/clifacet/examples/simple/
src/commands/greet.rs"] pub mod greet;` — the absolute path of the source file on the
building machine, baked into generated code. It compiles and the tests pass; it is also
machine-specific output, invisible to `rust-analyzer`'s module resolution, and it explains
nothing when it breaks.

**Errors land in generated code.** Renaming `List` to `Listing` produced:

```
error[E0425]: cannot find type `List` in module `crate::commands::list`
  --> /…/target/debug/build/clifacet-example-…/out/discovered.rs:11:33
```

The caret is in `OUT_DIR`. The command file the mistake is actually in is not mentioned.

**Mechanism 1's cost, counted.** 45 lines of build script, one `include!`, one `#[path]`
hack per module. decision-14's hand-written enum for the same two commands is 7 lines.
Autodiscovery starts winning around twenty commands, and the repository's reference
application has one.

**Mechanism 2 would need tracking hacks.** A proc macro can read `CARGO_MANIFEST_DIR` and
the directory at expansion time, which removes the `#[path]` problem but not the error
paths. Worse, cargo's dep-info does not see files a proc macro reads, so an edit to a
command file would not trigger a rebuild unless every file is also pulled in through
`include_bytes!` to register the dependency. Not compiled here; not recommended on the
strength of that alone.

**The alternative that works.** A `macro_rules!` expanding to `pub mod x;` resolves
relative to the **invocation** site, which is the opposite of `include!` and exactly what
is needed. A one-line-per-command macro built on that compiled and passed three tests:

```rust,ignore
// src/commands/mod.rs — the whole registration
clifacet::commands! {
    /// Say hello.
    greet: Greet => GreetInput,
    /// Count things.
    list: List => ListInput,
}
```

It emitted the module declarations, decision-14's `Commands` enum, the hand-written
`Request` enum decision-14 insists on, and the `TryFrom<Commands> for Request` dispatcher
that delegates to each leaf's `Facet::validate`. `cast greet --name ada` parsed and
validated; `render_help()` printed `greet  Say hello` / `list  Count things` from the doc
comments on the invocation lines. One trap found: `clap` strips the trailing period from
short about text.

## Decision

1. **The required layout is a file per leaf**, and registration is **one line per
   command** in `src/commands/mod.rs`, through a `clifacet::commands!` macro that emits
   decision-14's tree. It is a spelling, not new semantics: the same enum, the same
   `TryFrom`, the same per-leaf `Facet::validate`. No build script, no generated files, no
   absolute paths, and every error points at a line in the application's own source.
2. **Filesystem discovery is an opt-in for large CLIs, never the default.** If it is ever
   built it belongs in a separate published crate (`clifacet-build`) that an application
   calls from its own `build.rs`, with the measured costs documented next to it —
   `OUT_DIR` error paths, `#[path]` absolute paths, and a build script in every consumer.
   A third crate also means a third crates.io trusted-publisher registration; decision-7
   names only `clifacet` and `clifacet-macros`.
3. **`inventory`-style distributed registration is rejected**: it contradicts decision-1,
   adds a runtime dependency and supply-chain entry to a library, makes iteration order
   link order (so alphabetical help output becomes something the application has to sort
   for), and replaces the statically-known tree — what `CommandFactory`, the help
   snapshots and shell completion all read — with one assembled at run time.
4. **A proc-macro directory scan is rejected** for the tracking reason above.

## Consequences

- **`mod` resolution decides the ergonomics.** `pub mod x;` from a macro expands relative
  to the invocation site, so `clifacet::commands!` must be invoked in the module whose
  directory holds the files (`src/commands/mod.rs`). Documented, with a `trybuild` case for
  invoking it elsewhere.
- **An unregistered file is a warning, not a command.** Adding `greet.rs` without a line in
  `mod.rs` leaves an unused module that never parses a flag. That is the deliberate
  difference from a router that scans: the list is the map of the CLI, it appears in a diff
  a reviewer reads, and a command cannot exist without someone having written it down.
- **The doc comment on the invocation line becomes the subcommand's about text**, and
  `clap` drops its trailing period — a detail the macro cannot hide, so the guide says it.
- **Migration stays additive.** A hand-written enum, as in decision-14, keeps working: the
  macro emits exactly that shape, and an application can start with the enum and adopt the
  macro later, or never.
- **A tree that outgrows the list has a documented door.** Past roughly twenty commands
  the build-script helper is worth its cost, and because it must emit the same generated
  shape, moving to it changes `mod.rs` and nothing else.
