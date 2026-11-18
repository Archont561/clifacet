---
id: decision-22
title: Figment-backed layered configuration and command mapping
date: '2026-11-18 15:00'
status: proposed
---
## Context

In 0.5.0, Decision-19 item 5 (CLI-51) introduced configuration layering via
`App::settings(app, loader)`, and CLI-52 introduced command functions
(`#[clifacet::command] pub fn cmd(...)`). Under CLI-51, file settings are read by a
`Loader` implementation (such as `clifacet::settings::toml::Toml`, powered by `toml_edit`)
into flat `Entry` records, which are then injected into `clap::Command` as argument
defaults before parsing.

While this preserves strict CLI precedence (`flag > environment > file > built-in default`)
and Clap's argument validation, it has several notable limitations:
1. **Format rigidity:** It is essentially confined to TOML (or requires bespoke `Loader`
   implementations for each format).
2. **No multi-layer merging:** Standard patterns like cascading system-wide (`/etc/`),
   per-user (`~/.config/`), and per-directory (`.app.toml`) configuration files cannot be
   composed without manual file-reading glue.
3. **No profile switching:** Developers cannot easily switch between `default`, `development`,
   `staging`, and `production` profiles.
4. **Decoupled data structures:** The generated input types (`*Input`) created by
   `#[clifacet::command]` do not implement `serde::Deserialize`, so command functions cannot
   directly extract or be driven by typed configuration dictionaries.

[Figment](https://crates.io/crates/figment) is the de facto standard in the Rust ecosystem
for zero-friction, layered, hierarchical configuration management. This decision defines how
`clifacet` integrates Figment so that **each function representing a command maps
intuitively and automatically to a configuration section**, maximizing ergonomics for both
CLI authors and terminal operators.

---

## What Makes It Ergonomic for the End User?

To evaluate ergonomics properly, we distinguish between two end-user personas:
1. **The Terminal Operator:** The person running the CLI binary in a shell.
2. **The CLI Author:** The software engineer building the CLI with `clifacet`.

### 1. Ergonomics for the Terminal Operator

| Ergonomic Need | How Figment + clifacet Solves It |
| --- | --- |
| **Intuitive File Structure** | Sections in the config file mirror the command hierarchy: a top-level command `fn serve(...)` maps to `[serve]`, while a nested subcommand `db::migrate` maps to `[db.migrate]`. Operators never have to guess key placement. |
| **Strict, Predictable Precedence** | Operators expect flags to override environment variables, which override config files, which override defaults: `CLI flag (--port 9000) > Env (APP_SERVE_PORT=9000) > Config ([serve] port = 9000) > Code default (8080)`. |
| **Multi-Source Cascading** | Config can be split across `/etc/myapp/config.toml`, `~/.config/myapp/config.toml`, and local overrides, automatically merged by Figment. |
| **Profile Switching** | Operators can switch deployment profiles with one variable or flag: `FIGMENT_PROFILE=production myapp serve`. |
| **Interactive & TUI Pre-population** | In `--interactive` or `--tui` mode, values from Figment pre-fill questions and form fields so the operator can inspect and accept them with a single keystroke. |
| **Citing Error Locations** | When a config value is invalid (e.g. `port = "abc"`), diagnostics cite the exact file, line, and key, rather than failing silently or with a generic message. |
| **Provenance Auditing** | `clifacet`'s provenance model tells the operator where every value came from (e.g. `serve.port = 8080 [file: ~/.config/app.toml:5]`). |

### 2. Ergonomics for the CLI Author

| Ergonomic Need | How Figment + clifacet Solves It |
| --- | --- |
| **Zero-Boilerplate Mapping** | An author writes a standard command function: `#[clifacet::command] pub fn serve(#[clifacet(long)] port: u16)`. No separate `ServeConfig` struct or manual serde mapping is needed; the parameters automatically define the config schema. |
| **Automatic Serde Derivation** | When clifacet's `serde` feature is enabled, `#[clifacet::command]` generates `#[derive(serde::Deserialize, serde::Serialize)]` on the input struct (`ServeInput`), enabling direct extraction: `figment.extract_inner::<ServeInput>("serve")`. |
| **One-Line Runner Setup** | Wiring configuration into the application takes a single method call: `App::<Cli>::new().figment(figment).run()`. |
| **Context Injection for Deep Config** | If a command needs rich non-CLI configurations (e.g., connection pools, API tokens), it accepts `cx: &Figment` or `cx: &AppConfig` via clifacet's context parameter. |

---

## Architectural Comparison of Approaches

We evaluated three architectural paradigms for mapping command functions to Figment config:

### Paradigm A: Pure Serde Extraction (Figment as Master Resolver)
- **Mechanism:** Clap parses only raw CLI arguments into a sparse match map. Matches are
  converted into a Figment `Provider`. Figment merges files, env, and CLI matches, then
  deserializes directly into `CommandInput: Deserialize`.
- **Drawbacks:** Completely bypasses Clap's `ValueParser`, breaks clifacet's prompt validation
  and custom error renderers, and requires all argument types to implement `Deserialize`.

### Paradigm B: Figment as Defaults Provider (Clap-First Injection)
- **Mechanism:** Figment loads and merges all configuration sources. The active command's
  section (`[command]`) is read from Figment as string defaults and injected into Clap's
  `Arg::default_value(...)`. Clap then parses `argv`.
- **Benefits:** Keeps Clap as the single source of truth for argument validation, retains
  unified error messages and interactive prompt defaults, and requires no serde derives.
- **Drawbacks:** Cannot easily represent complex non-scalar nested structures in CLI arguments.

### Paradigm C: The Unified Hybrid Model (Proposed)
- **Mechanism:** Combine Paradigms A and B:
  1. **Clap Layering for Command Execution:** Figment supplies resolved dictionary values
     to `clifacet::settings` for the active command path, injecting defaults into Clap.
  2. **Serde Derivation on Generated Inputs:** `#[clifacet::command]` generates
     `#[derive(serde::Deserialize, serde::Serialize)]` on `*Input` structs under the `serde`
     feature, allowing direct `figment.extract_inner::<Input>("command")` whenever desired.
  3. **Context Injection:** When commands need arbitrary, deep, or non-CLI configuration,
     the command function takes `cx: &Figment` or `cx: &CustomConfig`.
  4. **Key Normalization:** Kebab-case CLI arguments (`listen-port`) and snake_case config
     keys (`listen_port`) are reconciled automatically.

---

## Proposed Design Details

### 1. Command-to-Config Mapping Rules
- **Leaf Command:**
  ```rust
  #[clifacet::command]
  pub fn serve(
      #[clifacet(long, default = 8080)] port: u16,
      #[clifacet(long)] host: String,
  ) -> Result<(), Failure>
  ```
  Maps to TOML/YAML section `[serve]`:
  ```toml
  [serve]
  port = 8080
  host = "127.0.0.1"
  ```
- **Nested Command:**
  Inside `commands!(db)`:
  ```rust
  #[clifacet::command]
  pub fn migrate(#[clifacet(long)] dry_run: bool) -> Result<(), Failure>
  ```
  Maps to section `[db.migrate]`:
  ```toml
  [db.migrate]
  dry_run = true
  ```
- **Global / Root Options:**
  Root options (or `Globals`) map to top-level keys in the config file:
  ```toml
  verbose = true
  ```

### 2. The `FigmentLoader` Seam
In `clifacet::settings::figment`:
```rust
pub struct FigmentLoader {
    figment: figment::Figment,
}

impl FigmentLoader {
    pub fn new(figment: figment::Figment) -> Self { ... }
}

impl clifacet::settings::Loader for FigmentLoader {
    fn load(&self, _bytes: &[u8]) -> Result<Settings, Error> {
        // Extracts sections matching command paths from the resolved Figment dictionary
        // and converts them into clifacet::settings::Entry records.
    }
}
```

### 3. Runner Ergonomics in `App`
```rust
App::<Cli>::new()
    .figment(
        figment::Figment::new()
            .merge(figment::providers::Toml::file("config.toml"))
            .merge(figment::providers::Env::prefixed("MYAPP_").split("__"))
    )
    .run();
```

---

## Consequences

- **Feature Boundary:** Figment integration is isolated behind an optional feature flag
  (`settings-figment`), keeping `clifacet`'s core lightweight and preserving its MSRV.
- **Serde Support:** Extends `clifacet-macros` to emit `Deserialize` and `Serialize` derives
  on generated input structs when `feature = "serde"` is active.
- **Audit & Vendoring:** Adding `figment` requires vendoring `figment = "0.10"` and its
  transitive dependency `uncased = "0.9"`. Both are MIT/Apache-2.0 licensed, satisfying
  the repository's `deny.toml` policy.
