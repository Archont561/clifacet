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

The application keeps what it should: parsing with `clap`, validation, which stream
the data goes to, and which exit code a failure costs.

## Status

Experimental, working toward `0.1.0`. The command core, prompts, the `Facet` derive
and the TUI are implemented and covered; the JSON frontend is reserved and does not
exist yet.

```sh
cargo run -p clifacet-example -- --name ada --repeat 2   # flags
cargo run -p clifacet-example -- --interactive           # a prompt
cargo run -p clifacet-example -- --tui                   # the full-screen form
```

## Workspace

| Crate | What it is |
| --- | --- |
| `clifacet` | the library: the command seam, the frontends, the interaction policy, the result contract |
| `clifacet-macros` | the `Facet` derive, re-exported as `clifacet::Facet` |
| `clifacet-example` | the reference application, its tests and runnable demos — a consumer of the published API, not published itself |

## Documentation

| Page | Read it when |
| --- | --- |
| [Getting started](docs/getting-started.md) | adding the crate and writing a first command |
| [Modes and the interaction policy](docs/modes.md) | deciding which mode runs, and what a run costs |
| [The companion derive](docs/derive.md) | giving an existing `clap` command validated input |
| [TUI composition](docs/tui-composition.md) | drawing your own field or screen |
| [Testing](docs/testing.md) | proving a promise without a terminal |
| [Contributing](docs/contributing.md) | the checks, hooks and layout of this repository |
| [Command core](docs/command-core.md) | the reference: seams, ownership, streams, exit codes, secrets |

The [docs index](docs/README.md) has the full list. Every page is checked in CI:
links resolve, ```` ```rust ```` fences are compiled as tests, and every documented
`clifacet::…` path names a public item.

## Develop

```sh
bash scripts/bootstrap.sh   # checkout at the remote head, environment, hooks, smoke check
pixi run --frozen quality   # repository, feature, platform, Rust and advisory checks
pixi run --frozen test      # both nextest groups
pixi run --frozen check-docs  # links, anchors, compiled snippets
```

Supported: the empty, `default`, `interactive`, `tui` and `json` configurations plus
all-features together, on Linux, macOS and Windows, with Rust 1.88 as the floor — each
one checked in CI rather than assumed. See
[Supported platforms and feature combinations](docs/development.md#supported-platforms-and-feature-combinations).

## License

MIT OR Apache-2.0.
