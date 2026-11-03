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

`#[clifacet(..)]` on a field is the whole argument in one attribute: its grammar, the
question that supplies it when no flag did, and the rules that turn it into input. The
macro splits the keys. Grammar goes into one `#[arg(..)]`, so `clap` parses exactly as
it would have and the help, aliases and value parsers stay its; everything else goes
into a field-level `#[facet(..)]`, which the `Facet` derive turns into a generated
`TryFrom` bridge and a generated `Collect::collect`.

```rust
use clifacet::clap::Parser as _;
use clifacet::policy::{Capabilities, Mode, Policy, Request};
use clifacet::prompt::{Answer, Collect as _, Scripted, Session};
use clifacet::Facet;

#[clifacet::command(input = Greeting)]
#[command(name = "greet", about = "Say hello")]
struct Cli {
    /// Who to greet.
    #[clifacet(
        long, short = 'n', alias = "nickname", value_name = "NAME",
        ask = "Who should I greet?", fill = "world", trim, non_empty,
    )]
    name: Option<String>,

    /// How many times to greet, from 1 to 8.
    #[clifacet(long, value_name = "COUNT", range = 1..=8, default_value_t = 1)]
    repeat: u8,
}

/// The typed input: never missing a name, never a blank one.
struct Greeting {
    name: String,
    repeat: u8,
}

// The flags: clap parses, the generated bridge normalises.
let cli = Cli::try_parse_from(["greet", "--nickname", "  ada  ", "--repeat", "3"])
    .expect("the forwarded grammar parses");
let input = cli.validate().expect("valid");
assert_eq!((input.name.as_str(), input.repeat), ("ada", 3));
// `Facet::command()` is clap's command: the help is built from the forwarded keys.
assert!(<Cli as Facet>::command().render_help().to_string().contains("-n, --name <NAME>"));

// No flag at a terminal: the declaration asks, and the answer meets the same rules.
let plan = Policy::up_to(Mode::Interactive)
    .resolve(Request::none(), Capabilities::terminal())
    .expect("prompts are permitted");
let mut session = Session::start(plan, || {
    Box::new(Scripted::new([Answer::Text(String::from(" grace "))]))
});
let input = Cli::try_parse_from(["greet"])
    .expect("parses")
    .collect(&mut session)
    .and_then(Facet::validate)
    .expect("valid");
assert_eq!(input.name, "grace");
```

The keys are the whole documented surface. Anything outside them is a compile error
naming the supported keys, rather than an argument that quietly means nothing:

| Kind | Keys | Becomes |
| --- | --- | --- |
| grammar, with a value | `alias`, `conflicts_with`, `conflicts_with_all`, `default_value`, `group`, `help`, `hide`, `id`, `ignore_case`, `num_args`, `required`, `requires`, `short_alias`, `value_delimiter`, `value_name`, `visible_alias` | `#[arg(key = …)]` — clap's builder methods, so `required = true`, as in clap |
| grammar, bare | `value_enum` | `#[arg(value_enum)]` |
| grammar, either | `action`, `default_value_t`, `env`, `long`, `short`, `value_parser` | bare (clap derives the value from the field) or `key = …` |
| grammar, translated | `range = 1..=8` | `value_parser = clap::value_parser!(T).range(1..=8)`, `T` the field type with `Option`/`Vec` peeled |
| asking | `ask = "…"` | `collect` asks when no flag supplied the value; the answer is parsed by the argument's own clap parser |
| asking | `confirm = "…"` | `collect` puts the set flag to the user; declining cancels the run |
| normalising | `fill = expr` | the bridge's value when neither a flag nor an answer supplied one |
| normalising | `trim`, `non_empty` | the bridge trims, then rejects blank text — or, on an `Option`, makes it absent |
| normalising | `secret` | the field must be a `Secret<_>`, so the credential cannot be printed |

The bridge applies its rules in one order — `fill`, `trim`, `non_empty`, `secret` —
whatever order they were written in, because trimming after the blank check would let
`"  "` through. `ask` and `confirm` run in `collect`, which only fills the parsed
command in; `Facet::validate` still produces the input, so an answer and a flag meet
exactly the same rules.

Rules that cannot hold are compile errors at the key, each with a golden `trybuild`
file:

- **One key, one meaning.** A key declared twice is an error, not last-one-wins.
- **`confirm` is for a `bool`, `ask` for an `Option`.** A value that cannot be missing
  cannot be asked for, and only a flag that is set can be confirmed.
- **`ask` needs a named option.** The answer is parsed as the flag's value; a positional
  argument cannot be asked for.
- **No `ask` on a `secret`.** No frontend hides an answer yet, so the credential would be
  echoed as it is typed.
- **A rule names the types it fits.** `trim` on a number or `secret` on a `String` says
  so, in those words.
- **A declared `bridge = …` owns normalization.** A `fill`/`trim`/`non_empty`/`secret`
  beside it is an error, because it would do nothing; `ask` and `confirm` still work.

The escape hatch is the field next door: a raw `#[arg(..)]` on the same field passes
through untouched, so a clap key this table has not grown yet stays available without
waiting for a release. `crates/clifacet/tests/ui/field_attribute_valid.rs` proves the two
compose.

`#[derive(clap::Parser)]` with plain `#[arg(..)]` keeps working exactly as it did, and
the same field rules are available there as `#[facet(ask = …, trim)]` on the field —
`#[clifacet(..)]` forwards to that attribute rather than generating anything itself, so
the two spellings cannot drift.

## Three ways to build the input

| Attribute | Effect |
| --- | --- |
| `#[facet(input = Type)]` | required; the type `validate` returns |
| no `bridge`, no field rules | the generated `TryFrom` copies fields by name, so a renamed or missing field is a compile error |
| no `bridge`, field rules | the same `TryFrom`, with each field's `fill`/`trim`/`non_empty`/`secret` applied on the way |
| `#[facet(bridge = path)]` | calls `path(cli) -> Result<Type, Failure>` instead, for input whose rules span fields |

Reach for the bridge when a rule is about more than one field — the reference
application's `--repeat` range check and its blank-credential rule both predate the
field rules and still live in one `fn` there. A rule about one field belongs on that
field.

## What the derive does not do

- **It does not parse.** `Facet::command()` *is* `<Cli as CommandFactory>::command()`,
  so aliases, defaults and value parsers are `clap`'s, unchanged.
- **It does not re-run the parser.** Input is built from the value that was already
  parsed. An answer to `ask` is the one value parsed later, and it goes through the
  argument's own `clap` parser rather than a second one.
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
