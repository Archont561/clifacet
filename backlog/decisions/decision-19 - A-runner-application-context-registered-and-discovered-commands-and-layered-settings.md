---
id: decision-19
title: >-
  A runner, application context, registered and discovered commands, and layered
  settings
date: '2026-11-08 01:57'
status: accepted
---
## Context

After 0.3.0 and the CLI-32/41/42 sprint, the maintainer asked a series of design questions
in one review: how a build helper would differ as `clifacet-build` or `clifacet::build` and
what it costs to build, how a minimal but powerful API should look, how `_helper.rs` files in
`src/commands/` would work, how commands receive the application's context, how settings
load from a file with the environment and flags on top, whether clap is needed at all, and
what facet offers instead. Each answer was measured or compiled in throwaway workspaces
outside the repository; the evidence is below. On 2026-11-08 the maintainer adopted every
suggestion ("adopt all suggestions and prepare repo for new sprint").

The review's starting point was that every application still writes the same pipeline by
hand. backlog-lite has 120 lines of `app.rs`, a 125-line `commands/mod.rs`, and touches
seven places to add a command; inside a tree `self` is the parsed clap struct, so a command
has nowhere to receive a dependency, which is why backlog-lite reads a global
`CLIFACET_BACKLOG_BIN` and the tree tests record runs in a `thread_local!`.

## Decision

1. **A runner owns the pipeline.** `clifacet::run::<Cli>()`, or
   `clifacet::App::<Cli>::new().policy(..).run()`, parses, asks, validates, builds the
   context, runs or previews, renders and exits with decision-3's codes, so `main.rs` is one
   call. `clifacet::Globals` gives every application `--yes` and `--dry-run`, and an
   application adds its own global options beside them. `run_with(argv, capabilities,
   frontend, out, err)` is the same pipeline for tests, and a `Render` trait decides how
   output is written. (CLI-46)
2. **Commands receive the application's context by shared reference.** `Handler<Cx>` has
   `handle(&self, input, cx: &Cx)`; a blanket impl makes every `Command` a `Handler` under
   any context, so `Command` and every existing implementation stay as they are. A planned
   twin plans with the context. `#[clifacet::subcommands(.., context = T)]` and
   `commands!(context = T)` thread it to the chosen leaf. The runner builds the context
   after validation, on first use, and never for `--help`, `--version`, a usage error or a
   declined question; failing to build it is an execution failure. Context is dependencies;
   input is what the user supplied — a handle never goes into `Input`, and never into a
   global. (CLI-47)
3. **Registration is one line per command.** `clifacet::commands!(list, show)` in
   `src/commands/mod.rs` declares the modules, the tree, its dispatch and, at the root, a
   `Cli` built from Cargo metadata with `Globals` flattened. A file stem is the command name
   in kebab case and its struct is in PascalCase. `#[clifacet::command]`'s `input` defaults
   to `Self`. Under `commands!` the `Request` is generated from the leaves' inputs; the
   `#[clifacet::subcommands]` attribute keeps the hand-written `Request` of decision-14.
   A group is a folder whose `mod.rs` holds its options and its own `commands!`. (CLI-33)
4. **Discovery is an opt-in in clifacet, not a crate.** `clifacet::build::commands()`,
   behind a `build` feature and called from the application's own `build.rs`, reports each
   folder's commands, groups and helpers through a versioned environment protocol and reruns
   when a folder changes; `clifacet::commands!()` with no list reads its folder's report.
   `_name.rs` and `_name/` are private helper modules, visible to their folder and the
   groups below it and public only through a re-export in `mod.rs`; a `_` folder is never a
   group. Moving between the list and discovery changes one call. (CLI-50)
5. **Settings layer as flag > environment > file > built-in default.** File values are
   injected as clap defaults, so clap's precedence and value parsers apply to them. `--config
   PATH`, `<APP>_CONFIG` and a default location choose the file and `--no-config` skips it;
   a named missing file is an error, a missing default file is not. Keys mirror the flags.
   The core takes format-agnostic entries and an optional feature ships a TOML loader.
   Provenance gains `Source::File`, and the environment is read through an injectable source
   so tests never mutate the process environment. (CLI-51)
6. **Functions can be commands**, once 1–3 exist: a parameter typed `&Cx` is the context,
   every other parameter is input. (CLI-52)
7. **clap stays (decision-2 is confirmed), on lean features; facet is deferred.** The
   library depends on clap without its default features, and an application that wants
   colour and suggestions enables them (CLI-44). facet/figue is re-evaluated when facet 0.50
   or later is stable and its rust-version is at or below clifacet's (CLI-54).
8. **A secret never shows its environment value in `--help`.** A field declared
   `#[clifacet(.., secret)]` gets `hide_env_values = true`, and the derive refuses a
   `#[facet(secret)]` field whose `#[arg(env)]` would show it. This extends decision-12's
   covered surfaces. (CLI-43, shipped in 0.3.1)

## Evidence

**Build helper, crate versus module** (2-core sandbox, CPU time, both variants with the
`tui` feature): a std-only `clifacet-build` against `clifacet::build` costs +1% on a clean
dev build (61.1 s → 61.9 s), +7% clean release (88.3 s → 94.5 s) and +12% clean check
(48.6 s → 54.4 s) — a fixed ~6 s, because the build dependency compiles clap a second time
for the host. Adding a command file rebuilt in 0.8 s and a no-op in 0.2 s for both. A
separate crate would be a third publish target, a trusted-publisher registration and a
version-skew risk; the versioned protocol keeps a later move to one line.

**Discovery without decision-15's objections.** The macro emits `mod x;` at its own
invocation site from lists the build script passed in, so there is no `include!`, no
`#[path]` absolute path and no error in `OUT_DIR`; no proc macro reads the directory, so
decision-15 item 4 stands. Compiled with nested groups: a helper used by its sibling and by
a nested group (`super::super::_task`), a group-private helper, a re-export through `mod.rs`.
A helper reached from outside its folder failed with E0603 at the caller's line, a syntax
error in a helper was reported in the helper file, an unused helper item warned as usual,
and a new `_new.rs` was picked up with no other edit. An earlier prototype had never
declared helpers, so a `_shared.rs` full of garbage still built — fixed by declaring them.
`Span::local_file()` compiled on stable and returns a path relative to the workspace root;
it is resolved against the compiler's working directory. Rust 1.88 is still to be proven by
the msrv job (CLI-50).

**Context.** Compiled across a library and an application crate: the blanket
`impl<C: Command, Cx: ?Sized> Handler<Cx> for C` coexists with an application's
`impl Handler<App> for Add`; a tree mixing both ran under one context; a context that could
not be built ran nothing; a type implementing both traits failed with E0119.

**Settings.** Compiled against clap 4.6.7 with a file, `APP_PORT`/`APP_HOST` and flags:
every precedence case behaved, `--config` worked before and after the subcommand and from
`APP_CONFIG`, a named missing file failed and a missing default file was skipped, and file
errors read `bad.conf:2: serve.port: invalid value '99999': 99999 is not in 0..=65535`.
Three clap quirks: `required` is checked before defaults apply, so a file value must lift
it; `ValueParser::parse_ref` is crate-private, so a one-argument scratch command runs the
real parser; and defaults must be `'static` without clap's `string` feature.

**clap's cost.** In a clean release build of `clifacet-example`, clap's own crates are
31.7 s of 143.9 s CPU (22%): `clap_builder` 24.1 s, colour and suggestions 3.5 s,
`clap_derive` and `heck` 3.1 s; `syn` and friends are needed by `clifacet-macros` anyway.
The lightweight alternatives (lexopt, argh, bpaf) have no runtime model of the arguments,
no record of where a value came from, or neither, and clifacet's prompts, form, provenance
and public API are built on exactly those. With clap's default features off, the whole
workspace compiled and all 585 tests passed.

**facet/figue** (crates.io and docs.rs on 2026-11-08): figue 4.0.5 needs Rust 1.90 and
facet 0.46; figue 5 and facet 0.50 are release candidates needing Rust 1.92, unchanged since
2026-08-20. figue's core is at least 58k lines (figue 22.4k, facet-core 19.3k,
facet-reflect 16.5k) against clap's ~20k. It brings layering, provenance, `MockEnv`,
`sensitive` and completions; it also brings a name clash with clifacet's own `Facet` trait.

**The help leak.** `#[clifacet(long, env = "PATH", secret)]` printed
`[env: PATH=/home/user/clifacet/…]` in `--help` before CLI-43.

## Decisions this changes

- **decision-1** — amended: registration stays explicit, and a directory the application
  opts into from its own build script counts as explicit; hidden global or link-time
  registration stays refused.
- **decision-2** — confirmed.
- **decision-12** — extended by item 8.
- **decision-14** — item 4 amended for the `commands!` path only.
- **decision-15** — item 1 un-deferred (CLI-33); item 2 reversed (item 4 above); items 3 and
  4 stand.
- **decision-18** — the refusal of configuration-file layering is lifted (item 5), and its
  statement that `Source::Config` exists is corrected: the enum never had it.
- **decision-7** — unchanged: no new crate is published.

## Consequences

- **Sequencing.** M11 ships 0.3.1 (CLI-43, CLI-44, CLI-45). M12 is the runner, the context
  and `commands!`, proven on backlog-lite and released as 0.4.0 (CLI-46, CLI-47, CLI-33,
  CLI-48, CLI-49). M13 is discovery, settings and function commands, released as 0.5.0
  (CLI-50 to CLI-53). CLI-54 waits for its trigger.
- **The public surface grows by about six items** for items 1–4: `run`/`App`, `Globals`,
  `Render`, `Handler`, `commands!` and `build::commands`; settings and function commands add
  their own.
- **Two ways to write a tree remain**: the `#[clifacet::subcommands]` attribute with a
  hand-written `Request`, and `commands!` with a generated one. The guide says when to use
  which.
- **Applications that relied on clifacet for clap's colour** get it back by enabling clap's
  default features themselves; most already do.
