# Discovered commands example

Demonstrates build.rs-free declarative command tree registration in `clifacet` (decision-25).

## How it works

1. `src/commands/mod.rs` registers commands and subcommands declaratively with pure Rust macros.
2. `src/lib.rs` exports the command tree root and entry points without `build.rs` or filesystem scanning.
3. `src/main.rs` dispatches the application.
