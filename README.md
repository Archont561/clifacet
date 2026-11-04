# Clifacet

[![CI](https://github.com/Archont561/clifacet/actions/workflows/ci.yml/badge.svg)](https://github.com/Archont561/clifacet/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/Archont561/clifacet/graph/badge.svg)](https://codecov.io/gh/Archont561/clifacet)
[![crates.io](https://img.shields.io/crates/v/clifacet.svg)](https://crates.io/crates/clifacet)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Archont561/clifacet)

**One command core, several presentation modes.**

A command takes validated input and returns data or a failure. It never writes a
stream and never reads a terminal, so the same command is reachable from flags, a
prompt, a full-screen form, or a screen the application draws — and testable without
any of them.

```rust
use clifacet::{Command, Failure};

struct Greet;

impl Command for Greet {
    type Input = String;
    type Output = String;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        Ok(format!("hello, {input}"))
    }
}
```

`clap` stays the grammar authority. `#[clifacet::command(..)]` wraps its parser and
the input bridge in one attribute, so the flags, the typed input and the command
cannot drift apart:

```rust
use clifacet::Facet;

#[clifacet::command(input = Greeting)]
#[command(name = "greet", about = "Say hello")]
struct Cli {
    /// Who to greet.
    #[arg(long, alias = "nickname")]
    name: Option<String>,
}

/// Validated input. Every value of it is runnable.
struct Greeting {
    name: Option<String>,
}
```

That expands to `#[derive(::clap::Parser, ::clifacet::Facet)]` plus
`#[facet(input = Greeting)]` in front of the item, so `#[derive(clap::Parser,
clifacet::Facet)]` on its own keeps working. `clap` remains a direct dependency,
because clap's derive emits code naming `clap::…`; this crate re-exports `clap` so the
version is the one the macro was built against.

The application keeps what it owns: which stream the data goes to, and which exit
code a failure costs.

## What is in the box

| Module | Answers |
| --- | --- |
| `clifacet::command` | what a command is: validated input in, data or a failure out |
| `clifacet::facet` | how a `clap` command becomes typed, validated input |
| `clifacet::policy` | which mode a run may use and which it prefers, resolved from a request, a policy and what the environment offers |
| `clifacet::prompt` | how a question is asked: a plain reader, an `inquire` dialog, or your own — and how to script the answers in a test |
| `clifacet::tui` | the full-screen form, its fields, and the two escape hatches for drawing your own |
| `clifacet::contract` | what a run may write where, and what each outcome costs |
| `clifacet::harness` | proof that a different frontend changed nothing else |
| `clifacet::secret` | a credential that cannot be printed by accident |
| `clifacet::provenance` | where a value came from, recorded without recording the value |

Frontends are features, so an application pays only for what it uses: `interactive`
(prompts, with an `inquire` dialog), `tui` (full screen), and a reserved `json`
boundary with no flag behind it yet. Prompts are a two-method seam — `ask` and
`confirm` — so the dialog is a default rather than a fixture: an application can hand
`Session` its own frontend and keep the policy, the streams and the exit codes.

## Status

Experimental. The command core, the interaction policy, prompts, the derive, the
`command` wrapper, one `#[clifacet(..)]` declaration per field and the TUI are
implemented and covered; the JSON frontend is reserved and does not exist yet.

**`0.2.0` is published.** [`clifacet`](https://crates.io/crates/clifacet) and
[`clifacet-macros`](https://crates.io/crates/clifacet-macros) are on crates.io, tagged
[`v0.2.0`](https://github.com/Archont561/clifacet/releases/tag/v0.2.0) — see the
[changelog](CHANGELOG.md) for what changed since `0.1.0` — and released by a manual,
authorized dispatch of `release.yml`: publishing stays a separate act from merging, so a
merge never puts a version on the registry by itself.

```sh
cargo run -p clifacet-example                            # asks for what the flags left out
cargo run -p clifacet-example -- --name ada --repeat 2    # flags
cargo run -p clifacet-example -- --interactive            # a prompt, or a refusal
cargo run -p clifacet-example -- --tui                    # the full-screen form
```

[`examples/simple`](examples/simple/README.md) is that application, with its own
README, its sources and its suite.

## Workspace

| Crate | What it is |
| --- | --- |
| `clifacet` | the library: the command seam, the frontends, the interaction policy, the result contract |
| `clifacet-macros` | the `Facet` derive, the `#[clifacet::command]` wrapper and `version!`, re-exported through `clifacet` |
| `clifacet-example` (in [`examples/simple`](examples/simple/README.md)) | the reference application, its tests and runnable demos — a consumer of the published API, not published itself |

## Documentation

| Page | Read it when |
| --- | --- |
| [Getting started](docs/getting-started.md) | adding the crate and writing a first command |
| [Modes and the interaction policy](docs/modes.md) | deciding which mode runs, and what a run costs |
| [The companion derive](docs/derive.md) | giving an existing `clap` command validated input, or wrapping both in one attribute |
| [TUI composition](docs/tui-composition.md) | drawing your own field or screen |
| [Testing](docs/testing.md) | proving a promise without a terminal |
| [Contributing](docs/contributing.md) | the checks, hooks and layout of this repository |
| [Command core](docs/command-core.md) | the reference: seams, ownership, streams, exit codes, secrets |

The [docs index](docs/README.md) has the full list. Every page is checked in CI:
links resolve, ```` ```rust ```` fences are compiled as tests, and every documented
`clifacet::…` path names a public item.

## Develop

```sh
bash scripts/bootstrap.sh          # checkout at the remote head, environment, hooks, smoke check
pixi run --frozen --no-install test-local   # the offline gate: format, lint, docs, tests, advisories
pixi run --frozen quality         # adds the checks that need the network: deny, audit, backlog, skills
pixi run --frozen check-docs      # links, anchors, compiled snippets
pixi run --frozen check-packages  # what `cargo publish` would upload, without publishing it
```

Supported: the empty, `default`, `interactive`, `tui` and `json` configurations plus
all-features together, on Linux, macOS and Windows, with Rust 1.88 as the floor — each
one checked in CI rather than assumed. See
[Supported platforms and feature combinations](docs/development.md#supported-platforms-and-feature-combinations).

## License

MIT OR Apache-2.0.
