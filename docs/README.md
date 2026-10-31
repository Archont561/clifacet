# Clifacet documentation

Clifacet is a Rust library for command-line presentation modes: one command core,
rendered through flags, prompts, a full-screen form, or a screen the application
draws itself. The core, the prompts, the companion derive and the TUI are implemented;
the JSON frontend is reserved and not implemented.

## Guides

| Page | Read it when |
| --- | --- |
| [Getting started](getting-started.md) | adding the crate and writing a first command |
| [Modes and the interaction policy](modes.md) | deciding which mode a run uses, and what it costs |
| [The companion derive](derive.md) | giving an existing `clap` command validated input |
| [TUI composition](tui-composition.md) | drawing your own field or screen |
| [Testing](testing.md) | proving a promise without a terminal |
| [Contributing](contributing.md) | working on the crate, its checks and its docs |

## Reference

| Page | What it records |
| --- | --- |
| [Command core and its seams](command-core.md) | the four layers, ownership, streams, exit codes, secrets, parity, and what is deliberately not implemented |
| [Development](development.md) | the task surface, supported platforms, MSRV, hooks, release and the sandbox |

Planning lives separately, under [`backlog/`](../backlog/): decisions, milestones and
tasks are the project's source of direction, not a rendered website. No documentation
site is required for 0.1.0.

Every page here is checked in CI: links and anchors resolve, ```` ```rust ```` fences
are compiled as tests, and every documented `clifacet::…` path names a public item.
See [Contributing § Documentation is checked](contributing.md#documentation-is-checked).
