---
id: CLI-52
title: Write a command as a function
status: Done
assignee: []
created_date: '2026-11-08 02:00'
updated_date: '2026-11-09 14:01'
labels: []
milestone: m-13
dependencies:
  - CLI-47
  - CLI-33
type: task
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 6, the last piece of the authoring surface: `#[clifacet::command] pub fn add(title: String, cx: &App) -> Result<usize, Failure>`. A parameter typed as a reference to the application context is the context; every other parameter is input, and the generated struct is what clap parses. Comes after the runner, the context and registration because it is sugar over all three.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A function annotated `#[clifacet::command]` becomes a command whose arguments are its non-context parameters, with doc comments as help and `#[clifacet(..)]` field keys on parameters
- [x] #2 A parameter typed `&Cx` receives the application context; more than one context parameter is a compile error that says why
- [x] #3 A function command registers and dispatches exactly like a struct command, explicitly and through discovery
- [x] #4 trybuild cases cover a non-`Result` return type and an unsupported parameter pattern, and the guide shows both forms side by side
- [x] #5 A bad `#[clifacet(..)]` on a parameter is reported at that parameter, not inside the expansion
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## The design, settled before code

The surface, with every rule that the review settled. The reasoning lives here because it
describes code that does not exist yet; it moves to `docs/` when the feature ships.

### The surface

A function annotated `#[clifacet::command]` is the whole leaf: grammar, help and body in one
item. `commands!(list, ..)` registers it with no change, because the macro generates the type
named after the file (`list.rs` -> `list::List`) that `commands!` already resolves.

With the application's context, the context is the last parameter:

```rust
#[clifacet::command]
pub fn list(
    #[clifacet(long, value_parser = parse_status, help = "Only tasks in this status")]
    status: Option<Status>,
    #[clifacet(long, help = "Only tasks whose dependencies are all done")]
    ready: bool,
    #[clifacet(long, help = "Print at most this many")]
    limit: Option<u32>,
    cx: &App,
) -> Result<Vec<Row>, Failure> {
    cx.store.list(status, ready, limit)
}
```

### Rules, and the reason for each

- **A bare parameter is a positional**, exactly as a bare struct field is. `limit:
  Option<u32>` with no attribute parses as a positional; a flag needs `long`. This is the
  rule that keeps the two forms from drifting. `docs/derive.md` already records that a
  positional cannot be asked or formed, so a function command with a bare parameter is
  real and deliberately less capable -- the same trade the struct form makes.
- **A `long` parameter carries `help`.** A parameter cannot have a doc comment (checked, see
  below), so `help` is the only place its description can live; `long` without it is a
  compile error rather than a flag with a blank description.
- **`&Cx` is the context only when `mod.rs` declares it**, and it must be last. `&App` is
  valid when the folder's line says `commands!(context = crate::App, ..)`. No inference from
  the type alone: without a declared context there is nothing to borrow, and guessing would
  turn a clear message into a borrow error. The cost, admitted: two files edited per command,
  so the error for an undeclared `&Cx` names the `mod.rs` line to add.
- **The parameter list generates both the parsed struct and the input type**, field for
  field. Nothing is restated. `new.rs` in `examples/backlog-lite` writes this by hand across
  three items and a bridge function today.
- **`input = T` survives as a name only**, not as a restatement: the parsed struct still
  comes from the parameters. It is for the case where `T` is a domain type the application
  already has. Because nothing then checks that `T` matches the parameters, a declared
  `input` with no struct-level `check` is refused -- `check` is where "every value of
  `Input` is runnable" gets a home. Open question for review: if the escape hatch earns
  nothing that `check` plus the generated type does not cover, it should not ship at all.

### Where the card's wording needs correcting

AC#1 says "doc comments as **params**". Doc comments on parameters do not compile --
verified, `error: documentation comments cannot be applied to function parameters`. The
honest reading: the **function's** doc comment becomes `about`/`long_about`, and a
**parameter's** help is `#[clifacet(help = "..")]`, which the field-key table already
forwards. Synthesising a `#[doc]` from elsewhere would be a second spelling for help that
does not match the struct form.

AC#2's "more than one context parameter is a compile error" stays, and gains two siblings:
a `&Cx` that is not last, and a `&Cx` in a folder with no declared context.

### The error table

Every message is written where the author can act on it -- on the offending parameter, or
naming the `mod.rs` line to change:

| Written | Error says |
| --- | --- |
| a function returning `String` | a function command returns `Result<_, Failure>` |
| `-> Result<String, io::Error>` | the error type is `clifacet::Failure`: convert with `?` |
| `x: &str` | a reference is not input -- take it by value, or use `&Cx` for the context |
| `a: &App, b: &App` | one context parameter: two would disagree about which is the context |
| `a: &App`, no `context = App` in `commands!` | no context is declared here -- add `context = crate::App` to `<path>/mod.rs` |
| `cx: &App, x: u32` | the context parameter must be last |
| `#[clifacet(long)]` with no `help` | a flag needs `help`: a parameter cannot have a doc comment |
| `fn list<T>(..)` | a generic command cannot be one grammar: clap needs a concrete type |
| `fn list(mut x: String)` | a parameter pattern is a name; the field is built by the parser |
| `#[derive(Debug)]` on a function | a function carries no derives; a command that needs them is a struct |
| `input = Filter` with no `check` | a declared input needs `check`: nothing else verifies it matches |

### Not in scope

- **No `planned` function form.** A planned command answers with a `Plan`; the signature would
  have to name it, and no one has asked. Additive later.
- **No `collect` on the function form.** Collection is a second function named by `collect =
  ..` in the struct form, and a function signature has nowhere natural for it. Applications
  that ask questions write the struct form; the two coexist in one folder.
- **No change to the struct form.**

### Cost and order

About 350 lines in `crates/clifacet-macros/src/function.rs`, one dispatch arm in
`crates/clifacet-macros/src/command.rs`, and the tests. Built in four provable slices:
(1) generate from parameters, proven by a function command and its hand-written struct
equivalent producing identical `--help` and identical parsed input; (2) context, with the
error naming the `mod.rs` line; (3) AC#3, one `examples/backlog-lite` leaf converted with its
neighbours left as structs; (4) the trybuild table.

### A bar to hold it to

Sugar that generates four impls is where error messages stop pointing at the author's code.
CLI-50's criterion was "every mistake is reported at the application's own file", and this
task should meet it: a bad `#[clifacet(..)]` on a parameter must fail at that parameter, not
inside the expansion.

## Built: the function form, slice by slice

The expansion lives in `crates/clifacet-macros/src/function.rs`, reached from
`command.rs` by dispatching on `syn::Item`. A function emits itself unchanged, plus the
type `commands!` resolves (`rename` -> `rename::Rename`), the input type
(`RenameInput`), and one impl: `Handler<Cx>` when there is a context, `Command` when
there is not — never both, because the library's blanket impl already makes every
`Command` a handler.

Decisions the build forced, each recorded where it can be argued with:

- **The `Facet` derive writes `validate`.** The generated struct carries
  `#[derive(Facet)]` and `#[facet(input = RenameInput)]` over fields built from the
  parameters, so the field-by-field bridge has exactly one generator. The struct form and
  the function form cannot drift because neither has a bridge of its own.
- **The context parameter is named `cx` or marked.** A bare `&str` last parameter was
  silently taken for a context by the type-only rule, and the resulting error was Rust's,
  not ours. The rule is now the parameter's name — `cx`, or `#[clifacet(context)]` for a
  name of the application's choosing — so a reference that is not a context gets the
  message about references.
- **Both generated types are public, and both kinds are exported.** clap's derive and the
  `Facet` derive expand in the caller's module, so a private field would make the command
  unparseable; the input has to be nameable because `commands!` writes
  `<path::Leaf as Facet>::Input`. The generated struct therefore *is* the exported name,
  which is what lets `pub use rename::Rename` work, with `validate` still the only way to
  an input.
- **`#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]` is added to the
  emitted function.** Both lints fire on code the macro requires: the `Result` is the
  command contract, and a `String` argument is clap's type rather than an allocation to
  avoid.
- **The function's doc comment moves to the generated struct**, rather than being
  re-derived into `#[command(about = ..)]`: clap then applies its own rule (a one-line
  about loses its closing period) to both forms identically, which the parity test checks.

### What is proven

- `crates/clifacet/tests/function_command.rs` (11 tests): identical `--help`, identical
  `about`, identical input and output against a hand-written struct; a bare parameter is
  a positional and is not accepted as a flag; the context reaches the function through
  `Handler`; a tree mixes a function leaf with a struct leaf and its `--help` lists both.
- `crates/clifacet/tests/ui/function_*.rs` (8 cases): not a `Result`, a pattern
  parameter, a reference that is not a context, a context that is not last, a flag with
  no `help`, `bridge = ..` on a function, a `skip` parameter carrying a rule, and a value
  type marked `context`.
- `examples/backlog-lite/src/commands/doctor.rs` is now a function, registered by the
  same `commands!` line as its seven struct neighbours, with `bl`'s whole flow suite
  passing unchanged: AC#3's "explicitly and through discovery" is the same call for both
  forms because both expose the same type name.

### Added while converting the examples

Two things the function form needed before an existing application could be moved onto
it, both found by converting `examples/simple` rather than by reasoning about it:

- **`skip` on a parameter.** A value clap parses but the input does not carry: the body
  receives it from the command instead. It is what lets the interaction flags stay on the
  command, where the application reads them, without becoming part of what the command
  runs on. A skipped parameter takes no other key — a rule on a value that never reaches
  the input would apply to nothing — and the emitted function carries
  `#[allow(unused_variables)]`, because not reading it is the declaration working.
- **The author's own `#[command(..)]`.** `name`, `version`, `about`, `long_about` and
  clap's other struct-level attributes are forwarded onto the generated struct, beside the
  doc comment and after the derives, and stripped from the `fn`: clap's helper attributes
  on a function are an error, and "used before it is introduced" besides.

The generated input also derives `PartialEq` and `Eq`, as a hand-written input does, so a
test can assert what a command line validated into without comparing field by field. And
the call into the function now names its parameters rather than the input's patterns: a
skipped parameter is an argument of `__clifacet_call` that comes from the struct, so the
call cannot be spelled from the input's destructuring alone.

### Left for the next task

The `commands!(context = ..)` declaration and the `&Cx` parameter are still two places a
command's context has to be written, and a mismatch between them is a trait error rather
than a message naming `mod.rs`. Making `mod.rs` one line —
`#[clifacet::commands(context = crate::App)]` on the module declaration — is the
follow-up that closes it, and it is additive to everything here.
<!-- SECTION:NOTES:END -->
