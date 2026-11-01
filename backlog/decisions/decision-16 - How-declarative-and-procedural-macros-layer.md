---
id: decision-16
title: How declarative and procedural macros layer
date: '2026-11-01 22:40'
status: proposed
---
## Context

decisions 13–15 propose three macro-shaped features: a field attribute for one command
(proc macro), a tree of commands (declarative), and one line per command to register it
(declarative). They have to coexist with what already ships — `#[clifacet::command]` is an
attribute proc macro, `Facet` is a derive proc macro — without two implementations of the
same rule.

Rust gives four expansion mechanisms, and they compose in one direction only because of
what each is allowed to emit:

| Mechanism | Can add to the item | Can emit other macros | Resolves relative to |
| --- | --- | --- | --- |
| attribute proc macro | anything, including derives and attributes | yes, the output is re-expanded | the tokens it received |
| derive proc macro | **impls only** — no attributes, no items | effectively no | the item |
| `macro_rules!` | items (as an item-position invocation) | yes | `$crate` for its own paths, the **invocation site** for `mod` |
| build script | nothing type-level: it writes a file something `include!`s | no | the filesystem, at build time |

## Measured

Compiled in this repository against the shipped crate, then reverted:

1. **A `macro_rules!` can emit an item carrying `#[clifacet::command(...)]`.** The
   attribute expands after the declarative expansion; the generated `Facet` impl and
   `clap::Parser` both resolve. This is the composition decision-15 depends on.
2. **A `macro_rules!` can emit a derive invocation** — `#[derive(clifacet::clap::Parser,
   clifacet::Facet)]` plus `#[facet(input = …)]` — with the same result.
3. **The obvious spelling of (1) does not compile.** With `$input:ty` (or `:path`), the
   attribute reports:

   ```
   error: `input` takes a path, for example `#[facet(input = my_module::item)]`
    --> examples/simple/tests/tmp_compose.rs:8:37
   ```

   A `ty`/`path` fragment reaches the proc macro as an opaque nonterminal group, and the
   derive's `Expr::Path` match rejects it. Forwarding the same argument as raw tokens —
   `($name:ident, $($input:tt)+) => { #[clifacet::command(input = $($input)+)] … }` —
   compiles and passes.
4. **Spans survive.** An error about a caller-supplied path points at the call site
   (`leaf_with_input!(Broken, MissingFromCallSite)` underlines `MissingFromCallSite`, not
   the macro body), and an error about a literal token in the macro body points at that
   line with a note naming the invocation. Composing the two macro kinds does not cost
   diagnostic quality.
5. **`crate::name!()` does not resolve for a `macro_rules!` that is not exported.**
   A macro shipped for downstream use must be `#[macro_export]`, which places it at
   `clifacet::commands!` — the crate root, outside the module it documents.
6. **A `macro_rules!` defined inside an expansion can be invoked in that expansion** on the
   current toolchain (1.88), so a macro may define and use helpers internally.

## Decision

Three layers, each owning one kind of decision, and no rule implemented twice:

1. **`macro_rules!` — the shape of the application.** `commands!` (one line per command →
   modules, the tree enum, `Request`, the `TryFrom` dispatcher) and `dispatch!` (a struct
   of command instances → a `Command` impl that routes typed input and converts outputs).
   These are structural: they emit items, they never inspect types, and their bodies are
   short enough to audit.
2. **Attribute proc macro — one item's declaration.** `#[clifacet::command]` turns field
   and item attributes into `#[arg(..)]`, `#[command(..)]`, derives and generated impls.
   This is the only layer that needs `syn`, spans and type syntax, so it is the only layer
   that gets them.
3. **Derive proc macro — the contract on a type.** `Facet` stays as it is: an impl for a
   type, nothing more.

The rules that make the layers composable:

- **A declarative macro never re-implements a proc macro's work.** `commands!` emits
  `#[clifacet::command(..)]` and lets the attribute expand; it does not inline the derive
  paths or the bridge body. One description, one expansion, one place to fix a bug.
- **Arguments that cross from declarative to procedural are forwarded as raw tokens**
  (`$($input:tt)+`), never as `:ty`/`:path` fragments — measured above. The alternative is
  to widen `facet(input = …)` to accept `syn::Type` as well as `Expr::Path`, which is
  recorded below as the preferred fix.
- **Shipped declarative macros use `$crate` for every internal path**; proc-macro output
  uses `::clifacet::`. This is not cosmetic: `$crate` survives a renamed dependency, while
  the derive's hardcoded `::clifacet::Facet` does not. The declarative layer is therefore
  the more robust surface, and the one to prefer where both are possible.
- **Everything a macro emits is public API.** An exported `macro_rules!` lands at the crate
  root, and its arms are as visible as a function. Helpers stay `#[doc(hidden)]` or fold
  into one macro with internal arms.

## Consequences

- **`facet(input = …)` should accept a `syn::Type`, not only an `Expr::Path`.** That is a
  small change to `parse_attribute` and removes the raw-token workaround from every caller
  of `commands!`. It needs a `trybuild` case for both spellings and is worth doing before
  decision-15's macro ships.
- **A derive can never grow attributes.** Anything decision-13 adds that must attach to a
  field — `ask`, `confirm`, `trim` — has to be read by the attribute macro, which is why
  `#[clifacet::command]` rather than a third derive remains the entry point.
- **The build-script option stays a separate kind of thing entirely** (decision-15): it
  produces tokens no macro can see, so it can only ever emit the same shape the
  declarative macro emits, never extend the derive.
- **Each layer needs its own tests.** `trybuild` for the attribute and derive layers (they
  are where misuse is diagnosed), and doctests plus one integration test per macro for the
  declarative layer, since `check-docs` compiles doctests and a macro with no doctest is a
  macro nobody documented.
