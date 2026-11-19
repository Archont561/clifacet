---
id: decision-26
title: Async command execution
date: '2026-11-19 19:00'
status: accepted
---
## Context

Commands are synchronous end to end: `Command::execute`, `Handler::handle`, `Planned::plan` and `Plan`'s work
(`FnOnce() -> Result`) all return values, not futures, and `App` holds its parts in `Rc`. Applications whose
dependencies are async (reqwest, sqlx, object_store, rmcp) must put a runtime in their context and `block_on` in every
command. That works (the polgeo prototype does it), but it leaves several things uncovered:

- **Ctrl-C** kills the process mid-write. There is no cooperative cancellation and no `Failure::cancelled` from an interrupt.
- **Long-running work** (downloads, bulk queries) has no progress channel. TUI, `--json` (CLI-72) and MCP (CLI-77) each need
  progress in a different shape.
- **`--dry-run` plans** can't do I/O while planning (e.g. HEAD requests to report download sizes).
- **The TUI loop** blocks on `crossterm::event::read()`, so a command can't run while the screen stays responsive.
- **Dynamic choices and completions** (CLI-78: "callbacks have access to `&self.cx`") are usually async lookups.
- **MCP (CLI-77)** uses rmcp, which depends on tokio unconditionally. Once that lands clifacet runs a runtime anyway, and
  `tools/call` should await async handlers directly, possibly concurrently.
- A nested runtime panics ("Cannot start a runtime from within a runtime") when an app built on `#[tokio::main]` calls `App::run()`.

## Decision

### 1. Async handlers, native, behind an `async` feature with no runtime dependency
```rust
pub trait AsyncHandler<Cx: ?Sized>: Send + Sync {
    type Input: Send;
    type Output: Send;
    fn handle(&self, input: Self::Input, cx: &Cx, run: &RunCx)
        -> impl Future<Output = Result<Self::Output, Failure>> + Send;   // AFIT, no async-trait crate
}
```
- `#[clifacet::commands]` detects `async fn` methods and implements the async trait. Sync and async leaves can be mixed in one tree.
- `Send` by default, because MCP and multi-thread executors need it. A `?Send` local variant (`LocalAsyncHandler`) covers single-thread and wasm.
- `AsyncCommand` / `AsyncPlanned` / `AsyncPlannedWith` mirror the existing traits.

### 2. A runtime boundary the application can choose
```rust
pub trait Executor {
    fn run<T: Send + 'static>(&self, fut: impl Future<Output = Result<T, Failure>> + Send + 'static, cancel: Cancel)
        -> Result<T, Failure>;
}
```
- `App::run()` stays synchronous and drives async leaves through the executor.
- Feature `tokio` ships `TokioExecutor`: current-thread by default, multi-thread opt-in, or wrapping a `Handle` the app already owns.
- `App::run_async(&self) -> ExitCode` and `run_with_async(..)` serve apps already inside `#[tokio::main]` and `#[tokio::test]`.
- Calling `run()` inside a runtime is a **`Failure`, not a panic**: "use `run_async`".
- `App` parts move from `Rc` to `Arc` under the feature (or the async path gets an `Arc` builder), because futures that
  borrow the context must be `Send`.

### 3. Async context construction, lazily
- `#[clifacet(context = build)]` accepts an `async fn(&Cli) -> Result<Cx, Failure>`, with an async `FromCli` twin.
- The context is built **only when a command runs**: not for `--help`, `--version`, validation failures or completions.
  So a bad flag never opens a connection pool or fetches a token.
- The lifecycle hooks (`before_run`, `after_success`, `after_failure`) get async variants.

### 4. Cancellation as part of the contract
- Every running command receives `RunCx`, which carries `Cancel` (a cancellation token) and the progress sink.
- **First Ctrl-C** trips `Cancel` and waits a configurable grace period. A command that returns `Failure::cancelled(msg)`
  keeps its message (e.g. "partial file kept for resume").
- **Second Ctrl-C, or the grace period running out** aborts the task.
- `Failure::cancelled` maps to exit code **130** in `ExitPolicy::standard()` (overridable). Streams are flushed. Output already written is never rolled back.
- Optional global `--timeout <dur>`, which produces `Failure::execution("timed out after …")`.

### 5. Progress: library-owned shape, frontend-owned look
```rust
pub trait ProgressSink: Send + Sync { fn total(&self, n: Option<u64>); fn advance(&self, n: u64); fn message(&self, m: &str); }
```
The library still writes no output of its own (decision-3). The frontend renders progress:

| Mode | Rendering |
|---|---|
| plain on a TTY | a stderr line |
| plain, not a TTY | nothing |
| TUI | a gauge |
| `--json` | NDJSON `{"event":"progress",…}` on stderr |
| MCP | `notifications/progress` |

### 6. Async plans
`AsyncPlan<'a, T>`: steps plus `FnOnce() -> BoxFuture<Result<T>>`, and `plan()` itself may be async.
`--dry-run` stays "the plan a real run executes".

### 7. TUI and interactive frontends
- **TUI:** the command runs as a task, and the TUI loop `select!`s on crossterm's `EventStream` and the task. Redraws and the
  cancel key stay live, and progress drives a widget.
- **Prompts:** async choice providers, `#[clifacet(choices = load)]` with `async fn(&Cx, &Partial) -> Vec<Choice>`.
  The same callback type serves CLI-78 dynamic completions, with a short timeout because shells wait for completions.

### 8. MCP alignment (amends CLI-77)
- `tools/call` awaits `AsyncHandler`s directly and runs sync handlers with `spawn_blocking`.
- Each call gets its own `Cancel`, tripped by MCP `notifications/cancelled`, and its own progress sink that maps to MCP progress.
- Concurrent calls share `Arc<Cx>`.

### 9. Testing stays "the same value, with the process taken out"
- `app.run_with_async(argv, caps, frontend, &mut out, &mut err).await` works under `#[tokio::test(start_paused = true)]`.
- `RunCx::for_test()` provides a recording progress sink and a manual cancel switch.
- Contract tests:
  - a cancelled leaf exits 130 with its own message
  - nested `run()` is a failure, not a panic
  - `--help` never builds the context
  - `--dry-run` never polls the work future

## Feature boundaries
| Feature | Adds | Dependencies |
|---|---|---|
| `async` | traits, `RunCx`, `Cancel`, `ProgressSink`, `AsyncPlan`, `Executor`, `run_async` | none (std futures) |
| `tokio` | `TokioExecutor`, signal handling, `spawn_blocking` bridge | tokio (`rt`, `signal`, `time`, `macros`) |
| `tui` + `async` | `EventStream` loop | crossterm `event-stream` |
| `mcp` | implies `async` + `tokio` | rmcp |

Sync-only applications compile exactly as today, and their tests don't change.

## Consequences
- clients (HTTP, DB, cloud storage) adopt clifacet without their own runtime plumbing. Ctrl-C, progress,
  dry-run and MCP behave the same in every mode.
- CLI-77 builds on this instead of inventing its own runtime bridge, so this decision lands before CLI-77.
- `Rc` to `Arc` (or a parallel `Arc` path) is the main internal change. Everything else is additive.

## Proposed tasks (milestone m-21)
| Id | Title | Depends on |
|---|---|---|
| CLI-79 | `async` feature: AsyncHandler/AsyncCommand/AsyncPlanned, RunCx, Cancel, ProgressSink | — |
| CLI-80 | Executor trait, TokioExecutor, `run_async`/`run_with_async`, nested-runtime failure | CLI-79 |
| CLI-81 | `#[commands]`/`#[group]` accept `async fn`; mixed sync/async trees; async context factory, lazy | CLI-79 |
| CLI-82 | Cancellation contract: Ctrl-C grace/abort, exit 130, `--timeout` | CLI-80 |
| CLI-83 | AsyncPlan and async `--dry-run` | CLI-79 |
| CLI-84 | Progress rendering per mode (plain, TUI gauge, NDJSON with CLI-72) | CLI-79, CLI-72 |
| CLI-85 | TUI EventStream loop; async choice providers; shared with CLI-78 completions | CLI-80, CLI-78 |
| CLI-86 | Amend CLI-77: MCP dispatch on AsyncHandler, per-call Cancel/progress | CLI-80, CLI-77 |
| CLI-87 | Acceptance example `examples/http-lite`: an async HTTP client CLI against wiremock (the async twin of backlog-lite) | CLI-81, CLI-82, CLI-84 |
