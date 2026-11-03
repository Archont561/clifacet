# The companion derive

`clifacet::Facet` connects an *existing* `clap` command to validated typed input. It
adds no parser and no second grammar: `clap` stays the authority, and the derive
names the one function that turns a parsed struct into an input the command can run.

```rust
use clap::Parser;
use clifacet::{Facet, Failure};

/// The command as it already exists.
#[derive(Debug, Parser, Facet)]
#[facet(input = Count, bridge = build_count)]
struct Cli {
    /// How many times to repeat.
    #[arg(long, default_value_t = 1)]
    repeat: u8,
}

/// The validated input: every value of it is runnable.
#[derive(Debug)]
struct Count(u8);

fn build_count(cli: Cli) -> Result<Count, Failure> {
    if cli.repeat == 0 {
        return Err(Failure::validation("--repeat must be at least 1"));
    }
    Ok(Count(cli.repeat))
}

let cli = Cli::try_parse_from(["app", "--repeat", "3"]).expect("clap parses it");
let count = cli.validate().expect("the bridge accepts it");
assert_eq!(count.0, 3);
```

## Or wrap both in one attribute

The derive and `clap`'s parser are two attributes that have to agree, which is easy to
get half right. `#[clifacet::command(...)]` is the pair, emitted as one:

```rust
use clifacet::clap::Parser;
use clifacet::Facet;

#[clifacet::command(input = Greeting)]
#[command(name = "greet", about = "Say hello")]
struct Cli {
    /// Who to greet.
    #[arg(long, alias = "nickname")]
    name: Option<String>,
}

/// The typed input, built field by field.
#[derive(Debug, PartialEq)]
struct Greeting {
    name: Option<String>,
}

let cli = Cli::try_parse_from(["greet", "--nickname", "ada"]).expect("clap parses it");
assert_eq!(cli.validate().expect("valid").name.as_deref(), Some("ada"));
```

It expands to the item exactly as written, preceded by
`#[derive(::clap::Parser, ::clifacet::Facet)]` and `#[facet(...)]`, and takes the same
keys. Because it emits the derive rather than a second implementation, the two cannot
drift, and `#[derive(clap::Parser, clifacet::Facet)]` still works on its own.

Two things it does not change:

- **`clap` stays a direct dependency.** `clap`'s derive expands to code naming `clap::…`,
  and an attribute cannot redirect that path. `clifacet` re-exports `clap`, so the
  version is the one the macro was built against, but the crate name has to resolve where
  the derive lands.
- **The grammar is still `clap`'s.** `#[command(..)]` and `#[arg(..)]` are read by
  `clap`'s derive, untouched; a mistyped flag is still a `clap` error at parse time.

A derive macro cannot add another derive to the item it is attached to — derives only
contribute impls — which is why this one is an attribute rather than a third derive. The
name lives in the macro namespace, so it does not collide with the `clifacet::command`
module.

## Declaring one field once

`#[clifacet(..)]` on a field is the same command declared more compactly: one attribute
carries the argument's grammar, so the `#[arg(..)]` it would otherwise need is written
for you. The macro forwards each key into a single `#[arg(..)]` on that field, so `clap`
parses exactly as before and the help, aliases and value parsers stay its.

```rust
use clifacet::clap::{CommandFactory as _, Parser as _};

#[clifacet::command(input = Greeting)]
#[command(name = "greet", about = "Say hello")]
struct Cli {
    /// Who to greet.
    #[clifacet(long, short = 'n', alias = "nickname", value_name = "NAME")]
    name: Option<String>,

    /// How many times to greet.
    #[clifacet(long, value_name = "COUNT")]
    repeat: u8,
}

/// The typed input, built field by field.
struct Greeting {
    name: Option<String>,
    repeat: u8,
}

let cli = Cli::try_parse_from(["greet", "--nickname", "ada", "--repeat", "3"])
    .expect("the forwarded grammar parses");
assert_eq!(cli.name.as_deref(), Some("ada"));
assert_eq!(cli.repeat, 3);
assert!(Cli::command().render_help().to_string().contains("-n, --name <NAME>"));
```

The keys are split by what they mean to clap, and the split is the whole documented
surface — anything outside it is a compile error rather than an argument that quietly
means nothing:

| Kind | Keys | Forwarded as |
| --- | --- | --- |
| value | `action`, `alias`, `conflicts_with`, `conflicts_with_all`, `default_value`, `default_value_t`, `env`, `group`, `help`, `id`, `ignore_case`, `num_args`, `range`, `requires`, `value_delimiter`, `value_name`, `value_parser` | `key = …` |
| flag | `hide`, `long_alias`, `required`, `short_alias`, `value_enum` | `key` |
| either | `long`, `short` | `key` (derive the name from the field) or `key = …` |

Two rules keep it honest:

- **One key, one meaning.** Declaring a key twice on a field is an error, not a
  last-one-wins resolution, because the pair would silently mean whichever came second.
- **The escape hatch is the field next door.** A raw `#[arg(..)]` on the same field
  passes through untouched, so a clap key this table has not grown yet stays available
  without waiting for a release; the test that proves the two compose lives in
  `crates/clifacet/tests/ui/field_attribute_valid.rs`.

`#[derive(clap::Parser)]` with plain `#[arg(..)]` keeps working exactly as it did — this
is additive. What the interaction keys (`ask`, `fill`, `confirm`) and the normalization
keys (`trim`, `non_empty`, `secret`) will do with the same declaration is decision-13;
they are not implemented yet, and naming one today is a compile error rather than a
silent no-op.

## Two ways to build the input

| Attribute | Effect |
| --- | --- |
| `#[facet(input = Type)]` | required; the type `validate` returns |
| no `bridge` | copies fields by name, so a renamed or missing field is a compile error |
| `#[facet(bridge = path)]` | calls `path(cli) -> Result<Type, Failure>` instead, for input that normalizes |

Use the bridge when the input is not a renamed copy of the struct: the reference
application uses it because `--name` normalizes from `Option<String>` to a trimmed
`String`, and a blank value has to be rejected. Both rules then live next to the data
they constrain, in one `fn`.

## What the derive does not do

- **It does not parse.** `Facet::command()` *is* `<Cli as CommandFactory>::command()`,
  so aliases, defaults and value parsers are `clap`'s, unchanged.
- **It does not re-run the parser.** Input is built from the value that was already
  parsed; constructs `clap` cannot re-validate stay out of scope.
- **It does not guess.** Unsupported shapes — a missing `#[facet(input = …)]`, an
  unknown key, an enum, an input missing a field the mirror would copy — are compile
  errors with a golden `trybuild` file, so the message says what to write instead.

## Testing it

Because the derive's promise is "the same command with typed input", the interesting
assertions are about parity, not about the macro: a flag alias, a `default_value_t`
and a custom `value_parser` all have to reach the same input as their canonical
spellings. `examples/simple/tests/facet_bridge.rs` and
`examples/simple/tests/features/facet_bridge.feature` are the worked examples.

The full reference, including the private-API rule that keeps the derive honest, is in
[Command core § The companion derive](command-core.md#the-companion-derive).
