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

## A command as a function

A leaf is often a small function with two or three arguments, and writing it as a struct
means writing the same thing twice: the `clap` struct for the grammar, and the input type
for the body. `#[clifacet::command]` accepts a function instead, and generates both from
the parameters:

```rust
use clifacet::Failure;

/// Add a task.
///
/// The title is stored as given.
#[clifacet::command]
pub fn add_task(
    #[clifacet(long, help = "The title to store")]
    title: String,
    #[clifacet(long, help = "Show what would happen")]
    dry_run: bool,
) -> Result<String, Failure> {
    Ok(format!("{title} ({dry_run})"))
}
```

The function is left as it was written. Beside it the macro emits the type a folder
registers — `add_task` becomes `AddTask`, the type the folder's registration looks for in
`add_task.rs` — the input type `AddTaskInput`, and the impls that connect them, so the
body receives its arguments by name and nothing is copied by hand. The doc comment moves
to the generated struct, where `clap` reads it: the first paragraph is the subcommand's
one-line description, the rest is its long help.

The same command, written as a struct, is the form the rest of this page describes:

```rust
#[clifacet::command(input = AddTaskInput)]
#[command(name = "add-task", about = "Add a task.")]
struct AddTask {
    #[clifacet(long, help = "The title to store")]
    title: String,
    #[clifacet(long, help = "Show what would happen")]
    dry_run: bool,
}

struct AddTaskInput {
    title: String,
    dry_run: bool,
}
```

Both register the same way and dispatch the same way, and a folder may mix them leaf by
leaf. The difference is what an author writes: the struct form declares the fields twice
and lets the derive bridge them, while the function form declares them once.

Three rules decide what a parameter means, and each one is the struct form's rule
applied to a signature:

- **A bare parameter is a positional**, exactly as a bare field is. `limit: Option<u32>`
  with no attribute is a positional argument; a flag needs `long`.
- **A flag needs `help`.** A parameter cannot carry a doc comment — Rust rejects one with
  "documentation comments cannot be applied to function parameters" — so `help` is the
  only place its description can live, and a `long` without it is a compile error rather
  than a blank line in `--help`.
- **A reference is not input.** `clap` builds the value from the argument, and a borrowed
  value would have nothing to borrow; take it by value.

A command that needs the application's context takes it as the last parameter:

```rust
# use clifacet::Failure;
# struct App { records: Vec<String> }
/// Count the records.
#[clifacet::command]
pub fn count(cx: &App) -> Result<usize, Failure> {
    Ok(cx.records.len())
}
```

The parameter is named `cx`, and the registration of the folder it sits in has to declare
the type — `commands!(context = crate::App, ..)` for a folder that lists its commands, or a
leaf that needs no line at all beyond its own signature. Neither is guessed from the
other: a `&App`
in a folder that declared no context is an error saying which line to add, and a `&App`
that is not the last parameter is an error saying where it belongs. A name other than
`cx` is available by marking the parameter `#[clifacet(context)]`.

A parameter carries the same rules a field does — `trim`, `non_empty`, `secret`, `ask`,
`confirm` — because the parameters are read by the same table. Two things behave
differently, and both are about a parameter being a value rather than a field:

- **`fill = ..` goes the other way.** On a field it turns an `Option<T>` into a `T` by
  supplying the fallback; on a parameter the type is already the `T` the body receives, so
  `fill` makes the *flag* optional and the derive fills it in. Write `name: String` with
  `fill = "world"`, not `Option<String>`.
- **A parameter's `skip` means the same thing.** The value is still parsed — `clap`
  builds it from the flag or the flattened group — but it is not part of the input, and
  the body receives it from the command instead. It is how a command carries what decided
  *how* to ask (the interaction flags, a `--dry-run`) without those becoming things the
  command runs on. A `skip` parameter takes no other key: a rule on a value that never
  reaches the input would apply to nothing, so it is refused rather than ignored.

The attribute takes three keys, and only these: `input` names the type the input is,
when it is one the application already has (the fields still come from the parameters);
`check` names a rule across the whole input; `collect` names a function that asks what
the declarations cannot describe. A fourth key is refused, because a function's
parameters are its declaration and `bridge` would be a second one.

The function form covers the leaves whose input is what the parameters say. A leaf
needs the struct form when the input is *not* that — when a value normalizes into a
different type (a bare `--status` filter that becomes "ask which" belongs to a bridge, not
to a parameter), when the bridge and the questions are one function, or when the command
is `planned`. `examples/backlog-lite` has both kinds side by side: `show`, `search`,
`start`, `done` and `doctor` are functions, and `list` and `new` are structs for exactly
those reasons.

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

// The flags: clap parses, the generated bridge normalizes.
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
| normalizing | `fill = expr` | the bridge's value when neither a flag nor an answer supplied one |
| normalizing | `trim`, `non_empty` | the bridge trims, then rejects blank text — or, on an `Option`, makes it absent |
| normalizing | `secret` | the field must be a `Secret<_>`, so the credential cannot be printed |
| input | `skip` | the bridge leaves the field out of the input; clap still parses it (not clap's `#[arg(skip)]`) |
| form | `form` | the field is listed in `Facet::form_fields`, in declaration order, for a generated TUI form |
| tree | `subcommands` | the field holds a [command tree](#nested-commands-a-tree-declared-where-its-commands-live), or an `Option` of one: clap's `#[command(subcommand)]`, the bridge converts the chosen leaf, `collect` lets it ask; it takes no other key |

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
- **`ask` and `form` need a named option.** An answer is parsed as the flag's value, and
  a form submits each field as its flag; a positional argument can be neither.
- **No `form` on a list.** A form field submits one answer, so a `Vec` (or an
  `Option<Vec<_>>`) would be cut to its first item; collect a list with
  `Session::collect_values`. A `bool` flag is fine: the form draws it as a toggle.
- **No `ask` on a `secret`.** The generated `collect` asks through `Session::ask_for`,
  which shows what is typed; a secret is asked for by hand with `Session::ask_secret`,
  which refuses on a frontend that cannot hide input instead of echoing it.
- **A rule names the types it fits.** `trim` on a number or `secret` on a `String` says
  so, in those words.
- **A declared `bridge = …` builds the whole input.** A `fill`/`trim`/`non_empty`/
  `secret`/`skip` or a struct-level `check` beside it is an error, because it would do
  nothing; `ask`, `confirm` and `form` still work.
- **A `skip` field takes no rule.** It never reaches the input, so a rule on it would
  have no value to apply to.

The escape hatch is the field next door: a raw `#[arg(..)]` on the same field passes
through untouched, so a clap key this table has not grown yet stays available without
waiting for a release. `crates/clifacet/tests/ui/field_attribute_valid.rs` proves the two
compose.

`#[derive(clap::Parser)]` with plain `#[arg(..)]` keeps working exactly as it did, and
the same field rules are available there as `#[facet(ask = …, trim)]` on the field —
`#[clifacet(..)]` forwards to that attribute rather than generating anything itself, so
the two spellings cannot drift.

## A rule of your own, a field that is not input, and the form

Three declarations are about the command rather than one value. `check = path` on the
struct names a `fn(&Input) -> Result<(), Failure>` that the generated bridge calls
last, on the input the field rules built — so it reads filled, trimmed values, and a
field rule that fails stops the run before it. `skip` keeps a field (a flattened group
of presentation flags, say) out of the input while clap still parses it. `form` lists a
field in `Facet::form_fields`, which is what a generated TUI form asks for:

```rust
use clifacet::clap::Parser as _;
use clifacet::{Facet, Failure};

#[clifacet::command(input = Booking, check = check_booking)]
#[command(name = "book")]
struct Cli {
    /// Who the booking is for.
    #[clifacet(long, fill = "guest", trim, non_empty, form)]
    name: Option<String>,

    /// How many seats: a `u8` parses 255, so the bound is the check's.
    #[clifacet(long, default_value_t = 1, form)]
    seats: u8,

    /// Presentation, parsed by clap and kept out of the booking.
    #[clifacet(long, skip)]
    loud: bool,
}

struct Booking {
    name: String,
    seats: u8,
}

fn check_booking(booking: &Booking) -> Result<(), Failure> {
    if booking.seats > 8 {
        return Err(Failure::validation(format!("--seats must be at most 8, got {}", booking.seats)));
    }
    Ok(())
}

let refused = Cli::try_parse_from(["book", "--seats", "9"]).expect("9 is a u8").validate();
assert_eq!(refused.err().map(|failure| failure.message().to_owned()).as_deref(), Some("--seats must be at most 8, got 9"));

let booking = Cli::try_parse_from(["book", "--name", " ada ", "--loud"]).expect("parses").validate().expect("valid");
assert_eq!((booking.name.as_str(), booking.seats), ("ada", 1));

// The form's field list, from the same declarations: `FormState::of::<Cli>()` takes it,
// and masks whatever `Cli::secret_fields()` lists — nothing, here.
assert_eq!(Cli::form_fields(), ["name", "seats"]);
assert_eq!(Cli::secret_fields(), [""; 0]);
```

A generated form submits each field under the flag clap knows it by — `--dry-run` for
a field `dry_run`, `-l` for a short-only one — so the answers parse exactly as typed
flags would.

## Nested commands: a tree declared where its commands live

A command with subcommands is a tree, and decision-14 makes it a declaration rather
than a hand-written enum plus a hand-written `match` that has to agree with it. The rule
is one sentence: **every level is an enum of leaf structs, and every leaf is a command.**

- The tree is an enum with `#[clifacet::subcommands(input = Request)]`, one variant per
  subcommand, each written `Name(Leaf)`. It is still clap's `Subcommand` — clap parses,
  and renders the help with the full path — and variant doc comments are the help text.
- Each leaf is an ordinary `#[clifacet::command(input = …)]` struct, with its own field
  declarations and rules. A subcommand that takes nothing is an empty `struct Leaf {}`.
- `Request` is written by the application: an enum with one variant per subcommand, of
  the same name, holding that leaf's input. The tree generates `TryFrom<Tree> for
  Request`, which validates the chosen leaf by its own `Facet`.
- The parent holds the tree in a field declared `#[clifacet(subcommands)]`. As an
  `Option`, an absent subcommand reaches the application as `None`, and the application
  picks a default leaf itself — the library never guesses one.
- Depth is a leaf that holds a tree of its own: the inner node is a leaf to the level
  above and a parent to the one below, so `cast db migrate` needs nothing new.
- With `output = Type`, the tree implements `Command` by running the chosen leaf with its
  input and converting the leaf's output with `Into`; with `planned` as well, it
  implements `Planned` instead, so a `--dry-run` describes the chosen leaf's plan.
  Validation consumes the parsed value, so a tree that runs keeps a `clone` of itself.

```rust
use clifacet::clap::Parser as _;
use clifacet::{Command, Facet, Failure};

#[clifacet::command(input = CastInput)]
#[command(name = "cast")]
struct Cast {
    #[clifacet(subcommands)]
    command: Option<Commands>,
}

struct CastInput {
    command: Option<Request>,
}

/// The first level: two subcommands, one of them a node.
#[clifacet::subcommands(input = Request, output = String)]
#[derive(Clone)]
enum Commands {
    /// Say hello.
    #[clifacet(alias = "hi")]
    Greet(Greet),
    /// Work on the database.
    Db(Db),
}

enum Request {
    Greet(GreetInput),
    Db(DbInput),
}

#[clifacet::command(input = GreetInput)]
#[derive(Clone)]
struct Greet {
    #[clifacet(long, fill = "world", trim)]
    name: Option<String>,
}

struct GreetInput {
    name: String,
}

/// An inner node: a leaf above, a tree below.
#[clifacet::command(input = DbInput)]
#[derive(Clone)]
struct Db {
    #[clifacet(subcommands)]
    command: DbCommands,
}

struct DbInput {
    command: DbRequest,
}

#[clifacet::subcommands(input = DbRequest, output = String)]
#[derive(Clone)]
enum DbCommands {
    /// Apply the pending migrations.
    Migrate(Migrate),
}

enum DbRequest {
    Migrate(MigrateInput),
}

#[clifacet::command(input = MigrateInput)]
#[derive(Clone)]
struct Migrate {
    /// Stop after this many.
    #[clifacet(long)]
    steps: Option<u8>,
}

struct MigrateInput {
    steps: Option<u8>,
}

impl Command for Greet {
    type Input = GreetInput;
    type Output = String;

    fn execute(&self, input: GreetInput) -> Result<String, Failure> {
        Ok(format!("hello, {}", input.name))
    }
}

impl Command for Db {
    type Input = DbInput;
    type Output = String;

    fn execute(&self, input: DbInput) -> Result<String, Failure> {
        self.command.execute(input.command)
    }
}

impl Command for Migrate {
    type Input = MigrateInput;
    type Output = String;

    fn execute(&self, input: MigrateInput) -> Result<String, Failure> {
        Ok(format!("migrated {}", input.steps.unwrap_or(1)))
    }
}

// Three levels down, clap still parses, and the help names the whole path.
let help = <Cast as Facet>::command()
    .try_get_matches_from(["cast", "db", "migrate", "--help"])
    .expect_err("help is clap's");
assert!(help.to_string().contains("Usage: cast db migrate"));

// The tree that runs is a clone; the request is the same parse, validated.
let cast = Cast::try_parse_from(["cast", "db", "migrate", "--steps", "2"]).expect("parses");
let tree = cast.command.clone().expect("a subcommand was given");
let request = cast.validate().expect("valid").command.expect("so it converted");
assert_eq!(tree.execute(request).expect("runs"), "migrated 2");

// No subcommand reaches the application, which picks its own default.
let bare = Cast::try_parse_from(["cast"]).expect("parses").validate().expect("valid");
assert!(bare.command.is_none());
```

A variant takes `#[clifacet(name = "…", alias = "…", visible_alias = "…", hide)]`, which
become clap's `#[command(..)]`; a raw `#[command(..)]` passes through for anything else.
A leaf whose questions the field declarations cannot describe — a choice, or a
confirmation that names the task — declares `collect = path` on its struct: a
`fn(Leaf, &mut Session) -> Result<Leaf, Failure>` that replaces the generated
questions, and the tree calls it when that leaf is chosen.
`examples/backlog-lite` is built this way.

What a tree gets wrong is a compile error, each with a golden `trybuild` file:

- **A leaf is a struct.** A variant with inline fields, a unit variant, or one holding
  two values says to write `Name(Name)` with a `#[clifacet::command]` struct instead.
- **`Request` matches the tree.** A missing variant is reported at the subcommand
  without one, a variant holding another input at the leaf, and a variant no subcommand
  produces at the tree's `input = …`.
- **One name, one subcommand.** A name or alias two variants answer to — a default name
  is clap's kebab-case of the variant — is reported at the second, naming it.
- **Known keys only.** An unknown key on the tree or on a variant is named in the error.
  The same is true of `planned` without `output`.
- **A `subcommands` field holds a tree.** Its type has to come from
  `#[clifacet::subcommands]`, and the field takes no other key.
- **A leaf converts.** A leaf without a `Facet`, or whose output does not convert into
  the tree's `output`, is reported at that leaf.
- **A declared `collect = …` asks every question.** A field's `ask` or `confirm` beside
  it is an error, because it would never be asked.

## One line on the module: `#[clifacet::registry]`

The smallest registration a folder of commands can have is a line where the folder is
declared, and nothing inside it (decision-21, CLI-58):

```rust,ignore
// src/lib.rs — the whole registration
#[clifacet::registry(
    output = crate::Said,
    name = "discovered",
    about = "A proof that a folder registers itself"
)]
pub mod commands;
```

```rust,ignore
// src/commands/mod.rs — everything that is not registration
pub use _shared::greeting;

pub struct Request { .. }
```

Four things come out of that one line, and each answers a question the folder used to
answer itself:

| Piece | What it does |
| --- | --- |
| the leaves' `mod` lines | one per file the build script reported, each with the `#[path]` of its file, because a `mod` a macro writes resolves beside the attribute |
| the tree and the request | `Commands`, and `Action` — one variant per leaf input — unless `request = Name` names it, so the folder may keep a `Request` of its own |
| `Input` and `Cli` | the root, with `clifacet::Globals` and the keys this line declares |
| the folder itself | its `mod.rs`, pulled in with `include!` beside the registration, so its `impl`s, its re-exports and its own types are written in its scope and mean what they meant |

What the folder gives up, and what it gains:

- **Its `mod.rs` may not start with `//!` or `#![..]`.** An `include!`d fragment cannot carry
  inner attributes, so the folder documents itself in `//` comments and its documentation
  goes on the module the attribute is written on. A file that starts with `//!` is refused
  with a message that says exactly that.
- **It is still a build script's job to say what is in the folder.** `clifacet::build::
  commands()` in `build.rs` is required, exactly as for `commands!()` with no list, and the
  `build` feature enables it.
- **A helper stays private.** The registry declares `_name.rs` beside the leaves and never
  re-exports it, so reaching one from outside its folder is `E0603`, and `mod.rs` decides
  what leaves.
- **`Action`, not `Request`, is the default name.** The folder is where a hand-written
  `Request` lives, so the generated enum has a name of its own; `request = Request` asks for
  the same name anyway.

The line is a `proc_macro_attribute` because a `macro_rules!` cannot reach a directory. What
makes it work is that a nested `mod` inside an `include!`d file resolves against *that*
file's directory, so the folder keeps its own resolution context while the registration runs
from `src/lib.rs`.

## Or a list inside the folder: `clifacet::commands!`


Declaring the tree is most of the work of a command with subcommands, but not all of it:
a folder also needs a `mod` line, a re-export, a variant in the tree and a second variant
in `Request` for every leaf. `clifacet::commands!` writes those, so the folder's `mod.rs`
is one call (decision-19 item 3, CLI-33). This is that file, `src/commands/mod.rs` —
an illustration rather than a compiled snippet, because it declares file modules and so
belongs to the crate, not to a test body:

```rust,ignore
use clifacet::clap::Parser as _;
use clifacet::facet::Facet as _;
use clifacet::Failure;

mod greet {
    use clifacet::Failure;

    /// Say hello.
    #[clifacet::command(input = Greeting)]
    #[derive(Clone)]
    pub struct Greet {
        /// Who to greet.
        pub name: String,
    }

    pub struct Greeting {
        pub name: String,
    }

    impl clifacet::Command for Greet {
        type Input = Greeting;
        type Output = String;

        fn execute(&self, input: Greeting) -> Result<String, Failure> {
            Ok(format!("hello {}", input.name))
        }
    }
}

/// The registry: one line, and the file stem is the command's name.
clifacet::commands!(greet, name = "greet-one", about = "One command, one file");

// What the call declared: the tree, the validated request, and the root.
let cli = Cli::try_parse_from(["greet-one", "greet", "ada"]).expect("parses");
assert!(matches!(cli.command, Some(Commands::Greet(_))));
let request = Request::try_from(cli.command.expect("a subcommand")).expect("validated");
assert!(matches!(request, Request::Greet(greet::Greeting { .. })));
```

What it declares, and why each name is fixed rather than configurable:

| Item | What it is |
| --- | --- |
| `mod <name>;`, `pub(crate) use <name>::<Name>;` | the leaf, one file per command |
| `enum Commands` | the tree, `#[clifacet::subcommands]`, one variant per module |
| `enum Request` | what the user asked for, validated: one variant per leaf input |
| `struct Input` | the root's validated input: `command: Option<Request>` |
| `struct Cli` | the root: the tree, `clifacet::Globals`, and the application's own |

- **The list is the whole registry.** Nothing is discovered, nothing registers itself by
  initializing a static, and the compiler checks what a list gets wrong: a module listed
  twice, an empty list, an unknown key. A leaf that is in the tree and missing from
  `Request` cannot happen, because both come from the same list.
- **Dispatch stays compile-time.** The tree is the same `#[clifacet::subcommands]` this
  attribute generates, so no subcommand reaches a command as an untyped argument.
- **A file stem is a name.** `search_history` becomes a `SearchHistory` struct that clap
  names `search-history`, exactly as clap would derive it; a leaf may still rename itself
  with `#[command(name = "..")]`.
- **`Request` is generated here only.** A tree declared with the attribute keeps the
  hand-written `Request` of decision-14. The two are the same tree; the difference is who
  writes the variant list.

The keys after the module names declare what the folder's commands share:

| Key | Effect |
| --- | --- |
| `globals = Type` | the application's own global options, a `clap::Args` type, flattened beside `Globals` and kept out of `Input` |
| `output = Type` | what every leaf's output converts into, so the root runs |
| `context = Type` | the application's [context](getting-started.md#what-a-command-depends-on): the root and the tree are `Handler<Type>`, and each leaf gets `&Type` |
| `planned` | the tree delegates `plan`, so a dry run shows the chosen leaf's plan |
| `request = Ident` | what the generated request enum is called, when the application has a richer `Request` of its own; `Request` by default |
| `root = { "field": Type }` | the root's own flags, global, folded into the generated input by name |
| `name = "…"`, `about = "…"`, `long_about = "…"` | the root's name and description, otherwise `CARGO_PKG_NAME` and `CARGO_PKG_DESCRIPTION` |
| `version_off` | no `--version`, for a root that forwards version queries elsewhere |

Groups are folders of their own, each with its own `commands!`: nested lists are not part of
this call. A nested path (`db::migrate`) reaches into a group the application declared
itself.

### Discovery, in either form

A folder can say what is in it instead of the call listing it, which is what makes dropping
a file into `src/commands/` the whole of adding a command:

```rust,ignore
// build.rs — the only line of setup, and it is the same for every application
fn main() {
    clifacet::build::commands();
}
```

```rust,ignore
// src/lib.rs — the registration, and the folder holds no call at all
#[clifacet::registry(output = crate::Said, name = "discovered")]
mod commands;
```

The same report can be read by a call inside the folder rather than by the attribute:

```rust,ignore
// src/commands/mod.rs — the list form, with no list
clifacet::commands!(output = crate::Said, name = "discovered");
```

Both halves are needed, and each has a job:

| Piece | Where it runs | What it does |
| --- | --- | --- |
| `clifacet::build::commands()` | the application's `build.rs` | scans `src/commands/` and every group folder under it, tells Cargo to rerun when the set changes, and reports what it found in `CLIFACET_COMMANDS` |
| `#[clifacet::registry]` | `src/lib.rs` | reads that report for the folder its module is declared beside, and emits the registration and the folder's `mod.rs` |
| `commands!()` | `src/commands/mod.rs` | reads that report, finds its own folder from the file it was written in, and emits the same `mod` lines a list would |
| the `build` feature | the application's `Cargo.toml` | enables the build-script half; off by default because a build script is its only caller |

The rules the folder follows:

- `<name>.rs` is a command named `<name>` whose struct is `Name`;
- `<name>/` is a group, which is a folder with its own registration — discovered, or written
  out;
- `_name.rs` and `_name/` are **helpers**: declared for the folder and the groups under it,
  never registered as a command, and `mod.rs` decides what they make public;
- `mod.rs` is the folder itself, never a command.

A helper is private to its folder by construction, so reaching one from outside is `E0603`
at the caller's line. Two other mistakes are reported where the mistake is: a syntax error
in a helper names the helper file, and a `<name>.rs` without its `Name` struct names that
file and the type it is missing.

Moving between the forms changes where the registration is written and nothing else:
`commands!(list, echo, db, …)`, `commands!(…)` and `#[clifacet::registry]` register the same
tree, and `examples/discovered` asserts exactly that. Discovery is a convenience, not a
different design — a folder that lists its commands stays fully supported, and is what an
application uses when a folder's members are not one-per-file.

`examples/backlog-lite` is registered by the attribute: its `bl` has seven subcommands, one
`--json` for the whole run, and one line on `mod commands` where three hand-kept lists used
to be.

### Which macro is needed?

The layering rule of decision-16, in one place. Every public macro belongs to exactly one
layer, and each is used in one situation:

| Macro | Layer | Expands to | Use it for |
| --- | --- | --- | --- |
| `#[derive(clifacet::Facet)]` | the grammar | `TryFrom<Cli> for Input`, `Collect`, `Facet` | an existing `clap` struct that needs validated input |
| `#[clifacet::command(input = …)]` | the grammar | `#[derive(clap::Parser, clifacet::Facet)]` plus `#[facet(..)]` | a new command: parsing, input and questions from one declaration |
| `#[clifacet::subcommands(..)]` | the tree | `#[derive(clap::Subcommand)]`, `TryFrom` into the application's `Request`, `Collect`, and `Command`/`Planned`/`Handler` as declared | a tree whose `Request` the application writes |
| `#[clifacet::registry(..)]` | the registration | the folder's `mod.rs`, `mod`s, re-exports, `Commands`, the request, `Input`, `Cli` | a folder whose files are its commands: one line on the module, nothing in the folder (CLI-58) |
| `clifacet::commands!(..)` | the registration | `mod`s, re-exports, `Commands`, `Request`, `Input`, `Cli` | a folder of commands that wants one line in the folder, and no hand-written `Request` |
| `clifacet::build::commands()` | the registration | a build script that reports its folders | discovery: a folder that registers itself, so adding a command is adding a file |
| `#[clifacet::planned]` | the context | `Handler<Cx>` for a `PlannedWith<Cx>` impl | a command that plans *with* the application's context |

Two rules hold across all of them: every path an expansion names is `$crate`- or
`::clifacet::`-rooted, so an application depends on `clifacet` alone; and a declaration in
the wrong place — a field attribute where a command is expected, `planned` on a tree with a
context, `registry` on a module that is not the folder — fails to compile with a message
that says what to write. Each of those is a golden `trybuild` case.

## Three ways to build the input

| Attribute | Effect |
| --- | --- |
| `#[facet(input = Type)]` | required; the type `validate` returns — any type the generated impls can name, see [below](#what-input-may-name) |
| no `bridge`, no field rules | the generated `TryFrom` copies fields by name, so a renamed or missing field is a compile error |
| no `bridge`, field rules | the same `TryFrom`, with each field's `fill`/`trim`/`non_empty`/`secret` applied on the way and `skip` fields left out |
| `#[facet(check = path)]` | the generated `TryFrom` ends by calling `path(&input) -> Result<(), Failure>`, for a rule across fields or one the keys cannot state |
| `#[facet(bridge = path)]` | calls `path(cli) -> Result<Type, Failure>` instead, for input the declarations cannot describe |

### What `input` may name

The input type is written into `type Input`, the generated `TryFrom` and `validate`'s
signature, so it may be any type that means the same thing in all three: a qualified
path (`crate::input::Greeting`), a generic type (`Labelled<String>`), an alias, or a
projection such as `<Greet as Family>::Input`. A lifetime in it (`Greeting<'a>`) has to
be declared on the command struct (`struct Cli<'a>`), which passes its generics to every
generated impl, or be `'static`. Without a `bridge`, the type has to name a struct,
because the generated bridge builds it as `Self { .. }`; with one, any type will do.

```rust
use clifacet::clap::Parser as _;
use clifacet::Facet;

/// A family of commands, each naming its input through one trait.
trait Family {
    type Input;
}

struct Greet;

impl Family for Greet {
    type Input = Labelled<String>;
}

struct Labelled<T> {
    name: T,
}

#[clifacet::command(input = <Greet as Family>::Input)]
#[command(name = "greet")]
struct Cli {
    #[clifacet(long, fill = "world")]
    name: Option<String>,
}

let input = Cli::try_parse_from(["greet"]).expect("parses").validate().expect("valid");
assert_eq!(input.name, "world");
```

What the generated impls cannot name is a compile error at the tokens at fault, not one
error per generated item: `Self` (inside those impls it is the command or the input
itself), a lifetime the command struct does not declare, the elided `'_`, `_` and
`impl Trait` (which leave the type unnamed), and — without a `bridge` — a type that is
not a struct path, such as a tuple.

A rule about one field belongs on that field, and a rule about the input belongs in
`check`. That is how the reference application is built: its `--repeat` range is
`check_input`, everything else is a declaration. Reach for a whole `bridge` only when
the input is not a field-by-field copy of the command at all.

## What the derive does not do

- **It does not parse.** `Facet::command()` *is* `<Cli as CommandFactory>::command()`,
  so aliases, defaults and value parsers are `clap`'s, unchanged.
- **It does not re-run the parser.** Input is built from the value that was already
  parsed. An answer to `ask` is the one value parsed later, and it goes through the
  argument's own `clap` parser rather than a second one.
- **It does not guess.** Unsupported shapes — a missing `#[facet(input = …)]`, an
  unknown key, an input type the generated impls cannot name, an enum, an input
  missing a field the mirror would copy — are compile
  errors with a golden `trybuild` file, so the message says what to write instead.

## Testing it

Because the derive's promise is "the same command with typed input," the interesting
assertions are about parity, not about the macro: a flag alias, a `default_value_t`
and a custom `value_parser` all have to reach the same input as their canonical
spellings. `examples/simple/tests/facet_bridge.rs` and
`examples/simple/features/facet_bridge.feature` are the worked examples.

The full reference, including the private-API rule that keeps the derive honest, is in
[Command core § The companion derive](command-core.md#the-companion-derive).
