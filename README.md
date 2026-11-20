<!--
  Generated file — do not edit README.md by hand.
  Source template: templates/README.md.j2
-->
# clifacet

<a href="https://crates.io/crates/clifacet"><img src="https://img.shields.io/crates/v/clifacet?style=flat-square&label=crates.io" alt="crates.io"></a>
<a href="https://docs.rs/clifacet"><img src="https://img.shields.io/docsrs/clifacet?style=flat-square&label=docs.rs" alt="docs.rs"></a>
<a href="https://github.com/Archont561/clifacet/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Archont561/clifacet/ci.yml?branch=main&style=flat-square&label=CI" alt="CI status"></a>
<a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/rust-1.88%20%E2%80%93%20current-orange?style=flat-square" alt="rust 1.88 - current"></a>
<a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=flat-square" alt="MIT OR Apache-2.0"></a>

**One command. Three terminal modes. Every presentation composed from the same declaration.**

A command is declared on a controller; an application is a value built from it. The same command runs
**noninteractive** — flags in, plain text for a person or structured output for a machine —
**interactive**, as a questionnaire on a terminal, or as a **TUI** full-screen interface.
Which one runs is decided once per run, from the flags, the application's policy and what
the terminal can offer.

```rust
use clifacet::contract::ExitPolicy;
use clifacet::policy::{Capabilities, Mode, Policy};
use clifacet::{Facet, Failure};

/// An application controller declares its global surface, context, and commands.
#[clifacet::app(name = "greet", about = "Say hello", commands)]
struct GreetApp {}

#[clifacet::commands]
impl GreetApp {
    /// The greeting command: parameters are arguments and flags.
    pub fn hello(
        &self,
        #[clifacet(long, short = 'n', help = "Who to greet", default_value = "world")]
        name: String,
        #[clifacet(long, help = "How many times", default_value_t = 1)]
        repeat: u8,
    ) -> Result<String, Failure> {
        Ok(format!("hello, {name} x{repeat}"))
    }
}

/// An application is a value built from parts that each own one decision.
fn app() -> clifacet::App<'static, GreetApp> {
    clifacet::App::<GreetApp>::for_cli()
        .policy(Policy::up_to(Mode::Interactive))
        .exit(ExitPolicy::standard())
}

/// `main` is the same value, run.
fn main_line() -> std::process::ExitCode {
    clifacet::run_cli::<GreetApp>()
}

/// And a test is the same value with the process taken out of it.
fn scripted() -> i32 {
    let mut out = Vec::new();
    let mut err = Vec::new();
    app().run_with(
        ["greet", "hello", "--name", "ada"],
        Capabilities::headless(),
        || Box::new(clifacet::prompt::Terminal::new(std::io::empty(), std::io::sink())),
        &mut out,
        &mut err,
    )
}

# assert_eq!(scripted(), 0);
```

That is the whole application. One declaration gives the flags, the validated input, the
command and the questionnaire's fields; `main` is one line; a test is the same pipeline
with buffers instead of a terminal. No stream, no terminal and no exit code appears in the
command, which is why all three modes reach the same code — and why the same declaration
survives being asked `--name ada`, being prompted for it, or being drawn as a field.

## What composes

Each part owns one decision and is applied where the application builds its run. Nothing
here is a global or a framework: an `App` is a value, so a test builds a different one.

| Part | Owns | Where |
| --- | --- | --- |
| `#[clifacet(..)]` | one argument's flags, prompt, form field and rules | on parameter or field |
| `#[clifacet::app]` / `#[clifacet::commands]` | declarative controller and commands in pure Rust | on struct / impl |
| `#[clifacet::command]` | command struct and typed input bridge | on the struct |
| `App` | the pipeline: parse, ask, validate, run, render, exit | `.run()` |
| `App::policy` / `mode_from` | which of the three modes a run may use, and which it prefers | composed |
| `App::settings` | a config file under the flags and the environment | composed |
| `App::exit` | what each outcome costs | composed |
| `run_with` | the capability, the frontend and the two streams | a test |

The seams themselves — streams, exit codes, ownership — are the
[command core](docs/reference/command-core.md); what each attribute accepts is
[the companion derive](docs/guides/derive.md).

## Start here

| To | Read |
| --- | --- |
| add the crate and write a first command | [Getting started](docs/guides/getting-started.md) |
| copy a working command | [`examples/simple`](examples/simple/README.md) |
| read every `#[clifacet(..)]` declaration | [The companion derive](docs/guides/derive.md) |
| declare controller commands and subcommands | [Nested command trees and controllers](docs/guides/derive.md) |
| layer flags, environment and a config file | [Settings from a file, under the flags](docs/guides/getting-started.md#settings-from-a-file-under-the-flags) |
| decide which mode runs, and what it costs | [Modes and the interaction policy](docs/concepts/modes.md) |
| draw your own field or screen | [TUI composition](docs/guides/tui-composition.md) |
| prove a promise without a terminal | [Testing](docs/guides/testing.md) |
| look up a seam, a stream or an exit code | [Command core](docs/reference/command-core.md) |

## The three examples

Each is a consumer of the published API, not a part of the library:

| Example | Shows |
| --- | --- |
| [`examples/simple`](examples/simple/README.md) | the reference application: one command, every seam, its own suite |
| `examples/discovered` | declarative command tree registration without build scripts |
| [`examples/backlog-lite`](examples/backlog-lite/README.md) | a wrapper around somebody else's CLI — the acceptance case |

## Install

```toml
[dependencies]
clifacet = "0.8.0"
```

Frontends are features, so an application pays only for what it uses: `interactive`,
`tui`, `serde` and `settings-toml`, off by default. [What each adds and
costs](docs/guides/getting-started.md#features).

## Documentation

`docs/` is the knowledge layer and it is checked, not remembered: every link and anchor
resolves, every Rust fence compiles, every documented `clifacet::…` path names a public
item, and the prose is linted for spelling and terminology.

| Directory | Holds |
| --- | --- |
| [`docs/guides/`](docs/guides/) | tasks, in the order a reader meets them |
| [`docs/concepts/`](docs/concepts/) | the model you need to predict behavior |
| [`docs/reference/`](docs/reference/) | the seams, the guarantees, the tables |
| [`docs/architecture/`](docs/architecture/) | how the repository is built, and why |
| [`docs/development/`](docs/development/) | working on it: checks, hooks, release |

[The docs index](docs/README.md) maps all of it, and [the documentation
policy](docs/concepts/documentation-policy.md) is what a change owes it.

## Status

Experimental. The command core, the mode policy, the prompts, the derive, the `command`
wrapper, the field declarations, the TUI, discovery and layered settings are implemented
and covered. The library writes no output of its own — machine-readable output is the
application's, built from [`Report`](docs/reference/command-core.md#known-unsupported-behavior)
— so there is no JSON mode, and no fourth mode is planned.

`0.8.0` is the version this README describes — [what each release
changed](CHANGELOG.md). Publishing is a separate, authorized act from merging.

## Develop

```sh
bash scripts/bootstrap.sh                        # environment, hooks, smoke check
pixi run --frozen --no-install tools run check   # everything a change must pass, offline
pixi run --frozen quality                        # adds the checks that need the network
```

`tools/` declares the Tasknest automation graph, and each script in
`tools/_lib/` still runs on its own. [The task graph](docs/architecture/repository.md)
explains the split; [Development](docs/development/development.md) has the checks.

Supported: the empty, `default`, `interactive`, `serde` and `tui` configurations plus
all-features together, on Linux, macOS and Windows, with Rust 1.88 as the floor.

## Built with

The repository's documentation and quality tooling:

| Tool | Renders or checks |
| --- | --- |
| [rumdl](https://github.com/Archont561/rumdl) | the structure and formatting of Markdown and MDX pages |
| [Vale](https://vale.sh) | the words: spelling, terminology, house style |

## Contributing

[Contributing](docs/development/contributing.md) has the checks, the hooks and the layout.
A change is done when the gate passes, the documentation it affects is updated or the
review records why not, and every command in the prose was run.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Archont561 — `https://github.com/Archont561/clifacet`
