# Discovered commands example

Demonstrates automatic command discovery in `clifacet`.

## How it works

1. `build.rs` invokes `clifacet::build::commands()`, scanning `src/commands/*.rs`.
2. `src/lib.rs` invokes `clifacet::commands!()` without explicit arguments, reading the discovered command registry.
3. `src/main.rs` dispatches the discovered application.
