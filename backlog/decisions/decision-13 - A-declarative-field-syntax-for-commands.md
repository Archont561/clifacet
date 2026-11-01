---
id: decision-13
title: One declarative attribute per field
date: '2026-11-01 21:30'
status: proposed
---
## Context

Two libraries in other ecosystems solve the problem clifacet solves, and both are more
compact to use than this crate is today. Their ergonomics are worth copying even where
their mechanisms cannot be.

**citty** (npm, 0.2.2) declares a command as one object: `defineCommand({ meta, args,
setup, cleanup, run })`. Each argument is one object too — `{ type, description,
required, default, alias, valueHint }` — where `type` is one of `positional`, `string`,
`boolean` or `enum`. Everything about an argument comes from that object: the flag, the
help text, the default, the short alias, the enum's options and whether it may be absent.
Nothing is declared twice, and `meta.name`/`meta.description` double as the usage text.

**typer** (PyPI, 0.27.3) declares a command as one function, with metadata attached to the
parameter default: `name: Annotated[str, typer.Option(help="…", prompt="…", min=1,
max=8, envvar="APP_LEVEL")] = "info"`. The annotation fixes the type, the default fixes
the fallback, and the `Option(...)`/`Argument(...)` call carries help, prompting,
bounds, environment fallback and whether the value is shown. The docstring becomes the
help. One parameter, one line, and the CLI is the function.

Clifacet arrives at the same destination from the other side: one command, four
frontends, validated input. But the reference application currently declares one
argument **three times** — once in `Cli` for clap, once in `ExampleInput` for the command,
and once in `collect()` for the prompt — plus a fourth time for the TUI form:

```rust,ignore
// 1. the flag
#[arg(long, short = 'n', alias = "nickname", value_name = "NAME")]
pub name: Option<String>,

// 2. the validated input it becomes
pub struct ExampleInput { pub name: String, /* … */ }

// 3. the question that supplies it when no flag did
let name = session.value(cli.name.take(), "Who should I greet?")?;

// 4. the list that makes it a form field
FormState::generated(&<Cli as Facet>::command(), &["name", "repeat", "level"])
```

Declarations 1 and 3 already say everything about `--name`. That is the gap: the
grammar is declarative and the interaction is imperative, so the interaction is what
drifts.

## Decision

One field attribute, `#[clifacet(…)]`, carrying both halves of an argument. The macro
splits its keys into two kinds and does the work for each:

| Kind | Keys | What happens |
| --- | --- | --- |
| grammar | `long`, `short`, `alias`, `value_name`, `value_parser`, `range`, `default_value_t`, `env`, `conflicts_with`, `required` | forwarded into an `#[arg(…)]` on the same field — `range` as clap's ranged value parser — so `clap` parses exactly as before |
| interaction | `ask`, `confirm`, `fill`, `form` | becomes a generated `collection` table and a generated form-field list |
| normalization | `trim`, `non_empty`, `secret` | becomes the generated `TryFrom` bridge that produces the declared `input` type |

```rust,ignore
#[clifacet::command(
    name = "example",
    about = "Clifacet's domain-neutral example command",
    input = ExampleInput,
)]
struct Cli {
    /// Who to greet. Defaults to `world`.
    #[clifacet(
        long, short = 'n', alias = "nickname", value_name = "NAME",
        ask = "Who should I greet?", fill = "world", trim, non_empty,
    )]
    pub name: Option<String>,

    /// How many times to greet, from 1 to 8.
    #[clifacet(long, value_name = "COUNT", range = 1..=MAX_REPEAT, ask = "How many greetings?")]
    pub repeat: u8,

    /// Fail on purpose, to demonstrate the execution-failure seam.
    #[clifacet(long, confirm = "Fail on purpose?")]
    pub explode: bool,
}
```

That single struct generates four things:

- `#[derive(::clap::Parser)]` and `#[arg(…)]` per field — unchanged grammar, unchanged
  help, unchanged value parsers. `clap` stays the authority (decision-2).
- `TryFrom<Cli> for ExampleInput` when `input` is named: fields with no normalization key
  are copied by name, so a renamed or missing field is still a compile error rather than
  a silent default; `trim`/`non_empty`/`secret` add their rule to that one field.
- `fn collect(self, session: &mut Session) -> Result<Self, Failure>`: asks the `ask`
  question for a field that no flag supplied, applies `fill` when nothing answered, and
  runs the `confirm` question for a flag that was set.
- `fn form_fields() -> &'static [&'static str]`: the TUI form's arguments, from the same
  declarations, instead of a hand-written list.

Escape hatches stay open in both directions: a raw `#[arg(…)]` on a field is passed
through untouched for anything the key table does not cover, and an unknown
`#[clifacet(…)]` key is a compile error that lists the supported ones.

### What is deliberately not copied

- **citty's `--no-` negation and camelCase access.** Both are runtime string conventions.
  In Rust the field name is the camelCase access, and `--no-x` on every boolean would
  change the grammar for every application.
- **typer's coercion from annotations.** Rust has the types; `clap`'s `value_parser` is
  the coercion, and a second one would be a second grammar.
- **citty's `setup`/`cleanup` hooks.** `Command` already separates collection from
  execution, and `Session::run` collects before it executes. A hook pair would be a
  second lifecycle with the same name.
- **Sub-commands.** citty nests them and typer has groups; both are worth doing, neither
  is part of this decision.

## Consequences

- `ask` on a field whose `value_parser` is an inline expression is a compile error: the
  generated `collect` parses an answer with the same parser the flags use, so the parser
  has to be a path it can name. This is the "does not guess" rule, extended to interaction.
- The reference application loses `collect()` and the form's argument list and keeps
  `ExampleInput`, which is the type that makes "every value of it is runnable" true —
  that guarantee is clifacet's reason to exist and is not traded for brevity.
- Every key needs a doc row and a `trybuild` case, for the same reason the existing
  derive has golden files: an attribute that fails at runtime is an attribute nobody
  trusts.
- Migration is additive. `#[derive(clap::Parser, clifacet::Facet)]` with `#[arg(…)]` keeps
  compiling exactly as it does now, so the example can move first and the guides follow.
