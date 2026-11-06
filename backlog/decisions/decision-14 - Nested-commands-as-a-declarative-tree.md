---
id: decision-14
title: Nested commands as a declarative tree
date: '2026-11-01 21:45'
status: accepted
---
## Context

decision-13 gives a *leaf* command one declarative attribute per field. It says nothing
about a command that has children, and the current `#[clifacet::command]` rejects an enum
outright with "clap's subcommand enums have no fields to copy" — which is true of a bridge
and false of a tree.

Both libraries this crate is learning from have an answer, and both are short:

- **citty**: `subCommands: { sub, db }` inside `defineCommand`, values being commands or
  `() => import(...)` for lazy loading; `meta.alias` and `meta.hidden: true` per child.
- **typer**: `app = typer.Typer()` and either `@app.command()` per function or
  `app.add_typer(sub_app, name="db")`; groups nest arbitrarily.

Clifacet needs the same expressiveness with one constraint neither library has: a
subcommand cannot be resolved dynamically at run time, because `clap` builds the whole
command tree before parsing. Whatever the syntax, the tree is static and compiled in.

## Decision

Two item kinds, two attributes, same rule underneath: **a leaf is a struct, a tree is an
enum, and the tree's variants are leaves.**

```rust,ignore
/// The whole CLI: this application's own flags, plus the tree.
#[clifacet::command(
    name = "cast",
    about = "Clifacet's example CLI",
    input = Request,
    output = ExampleOutput,
)]
pub struct Cast {
    /// The interaction flags — global, so they are accepted at any depth.
    #[clifacet(flatten)]
    pub interaction: InteractionFlags,

    /// The tree below this command.
    #[clifacet(subcommands)]
    pub command: Commands,
}

/// One variant per subcommand. Each leaf is an ordinary struct command.
#[clifacet::subcommands(input = Request)]
pub enum Commands {
    /// Say hello.
    Greet(Greet),
    /// Inspect what was greeted.
    #[clifacet(alias = "ls")]
    List(List),
}

#[clifacet::command(input = GreetInput)]
struct Greet {
    #[clifacet(long, ask = "Who should I greet?", fill = "world", trim, non_empty)]
    name: Option<String>,
}

#[clifacet::command(input = ListInput)]
struct List {
    #[clifacet(long, ask = "How many entries?")]
    limit: Option<u8>,
}
```

The expansions, all of them mechanical:

| Item | Emits | Why |
| --- | --- | --- |
| `#[clifacet::command]` on a struct with a `subcommands` field | `#[derive(::clap::Parser)]`, `#[command(..)]`, `#[arg(..)]` / `#[command(flatten)]` / `#[command(subcommand)]` per field | unchanged grammar; `clap` still parses and renders help |
| `#[clifacet::subcommands]` on an enum | `#[derive(::clap::Subcommand)]`, variant doc → `about`, `alias`/`hidden` → the same `#[command(..)]` keys | citty's `meta.alias` and `meta.hidden`, with clap's spelling |
| the tree's `TryFrom` | one match arm per variant, calling that leaf's own `Facet::validate` | the leaf owns its rules; the tree owns only the choice |
| the tree's `Command` impl | `type Input = Request`, `type Output = X`; `execute` matches the input and delegates | a dispatcher with no logic of its own |
| the tree's `collect` | match the variant, call that leaf's generated `collect` | one `Session`, so one plan and one `--yes` for the whole run |

`Request` is written by hand, and that is deliberate:

```rust,ignore
pub enum Request {
    Greet(GreetInput),
    List(ListInput),
}
```

It is the type the application matches on when it renders, so hiding it behind a
generated name would move the seam, not remove it. The macro checks it: a variant that
does not correspond to a subcommand, or one whose payload is not that leaf's input, is a
compile error rather than a branch that never runs.

### Shape of the root

A command with children and flags of its own is a **struct** with a
`#[command(subcommand)]` field. A CLI whose root has no flags can be the enum alone —
`clap` 4.6.7's `Parser` derive handles enums (`Data::Enum` in `clap_derive`) — but this
crate's reference application has three global flags, so it takes the struct shape.
Both were compiled before this paragraph was written; see *Verified*.

### One plan per run

The interaction mode is resolved from the root's flags before any leaf is chosen
(decision-3), and the flags are `global = true`, so `cast --yes greet` and
`cast db list --tui` reach the same `Plan`. A leaf cannot change the mode, and a leaf's
questions are asked under the plan the root already settled. That keeps the parity
property: the same subcommand run with flags, prompts or a form is the same command.

### Nesting

Depth is the same rule applied to a variant: a leaf that itself declares `subcommands` is
a tree, and `clap` accepts it at any depth. Nothing in the macro special-cases depth 2.

### The form

Each leaf generates its own field list, so `cast db list --tui` opens that leaf's form.
The root generates no form: a form is a screen for one command's questions, and the root
has none of its own — it has a choice to make. That is the picker below.

## What is deliberately out of scope

- **Lazy or dynamic loading.** citty's headline feature is impossible here: `clap` needs
  the whole tree to parse argv. `clap`'s `external_subcommand` is the only door to runtime
  dispatch, and it hands back untyped `OsString`s, which breaks the one promise this crate
  makes — every value the command receives is validated. Excluded, not deferred.
- **A prompt for which subcommand.** It is the one place clifacet could beat both
  libraries — neither asks — and it costs a third method on `Prompter`
  (`choose(&self, question, choices) -> Result<usize, Failure>`), which means `Terminal`,
  `Dialog`, `Scripted` and `Answer` all change. Worth doing; not inside this decision.
- **Per-variant policy, exit codes or streams.** decision-3 keeps those application-owned
  and run-wide; a tree does not get a second contract.
- **A generated `Request` enum.** Rejected above: the input type is the seam, and
  decision-2's rule is that the seam is named.

## Consequences

- **The `Command` trait needs one output type for the tree.** Leaves may produce different
  outputs; `output = X` on the tree requires `impl From<LeafOutput> for X`, and the
  generated arms end in `.map(Into::into)`. That is the honest cost of "the command
  returns data, never writes" — citty and typer dodge it by printing inside handlers.
  Erasing to `Box<dyn Any>` is rejected: it would move the type error from the macro to
  the application.
- **A tree adds four generated impls and a variant-level attribute.** Each needs a
  `trybuild` case for its misuse: a `subcommands` field whose type is not a
  `#[clifacet::subcommands]` enum, a variant whose leaf has no `input`, a `Request` variant
  with no subcommand, and a leaf output that does not convert.
- **`--help` and a bare invocation.** A missing subcommand is
  `DisplayHelpOnMissingArgumentOrSubcommand`, which `clap` streams to **stderr** with
  status 2 — so `cast` alone is a usage outcome, not help on stdout. The application
  decides what that costs; the example already maps clap's own verdict.
- **Sequencing.** This builds on decision-13's field attribute, so it lands after CLI-28:
  each leaf is an ordinary struct using it, and the tree adds only the choice.
- **Where the enum is written.** decision-15 keeps this shape and proposes a macro that
  emits it from one line per command, so a `Commands` enum is generated rather than typed
  out. The variants, the `Request` type and the delegation below are unchanged either way.

## Verified

Compiled against `clap` 4.6.7 before writing this, in a temporary test that was removed
afterwards; all six assertions passed:

1. `global = true` flags are accepted **before and after** a nested subcommand —
   `cast --yes greet` and `cast db list --tui` both reach the root's flag struct.
2. A tuple variant holding a `clap::Args` struct with its own `#[command(subcommand)]`
   parses at depth 2, and `matches.subcommand()` twice recovers `("db", "list")`.
3. `#[derive(Parser)]` on an enum works as a root with no flags of its own.
4. A bare invocation with a required subcommand yields
   `DisplayHelpOnMissingArgumentOrSubcommand`, not `MissingSubcommand`; `clap`'s
   `use_stderr()` sends that kind to stderr with status 2.

## Acceptance note (2026-11-06)

Accepted by the maintainer on 2026-11-06, with four amendments. The Decision above is
left as it was proposed; where the amendments differ, they win.

1. **The subcommand may be optional.** A root may hold `Option<Commands>`; choosing a
   default leaf when it is absent stays the application's, as `examples/backlog-lite` does
   with `list`. A required subcommand keeps clap's bare-invocation behaviour described
   under *Consequences*.
2. **Every variant wraps one leaf struct** (`Greet(Greet)`). An inline-field variant
   (`Show { id }`) or a unit variant (`Doctor`) is out of scope and a compile error that
   says to write a struct; `backlog-lite`, which has both today, measures the cost when it
   moves to the tree.
3. **The subcommand picker stays out of this decision, but is no longer expensive.** The
   *Form* section's "that is the picker below" contradicted the out-of-scope list; the
   list wins. Its stated cost — a new `Prompter::choose` — has since shipped with CLI-30
   (`Session::choose`), so a picker is a cheap follow-up, proposed on its own.
4. **`Request` is hand-written, and the generated code checks it.** decision-15's
   measured macro generated `Request`, which contradicts *A generated `Request` enum* above;
   this decision's rule stands. The generated conversion and dispatch match every variant
   of both enums, so a missing, extra or mistyped `Request` variant fails to compile.

Sequencing: after CLI-31 and the 0.3.0 release, because CLI-31 adds `Command::plan` and
the tree's generated `Command` impl delegates every method of the trait. CLI-32 carries
it, and moves `examples/backlog-lite` — the repository's only tree — rather than
`examples/simple`, which has no subcommands.
