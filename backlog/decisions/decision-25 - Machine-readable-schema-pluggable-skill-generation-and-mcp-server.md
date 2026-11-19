---
id: decision-25
title: Machine-readable schema, pluggable skill generation, and MCP server
date: '2026-11-19 14:30'
status: accepted
---
## Context

Following Decision 24 (Declarative class and controller authoring, lazy context, and lifecycle hooks), `clifacet` applications define their command structure as strongly typed Rust structs and `impl` blocks with direct `&self` access.

This declarative foundation enables a major architectural evolution:
1. **The `build` API is obsolete:** In Decision 19 & CLI-50, `clifacet::build::commands()` scanned `src/commands/*.rs` in `build.rs` and emitted an environment protocol (`CLIFACET_COMMANDS`) to work around proc-macro filesystem access restrictions. With declarative controller structs, all commands and subcommands are declared in Rust code. `build.rs` directory scanning is unnecessary overhead that inflates build times and complicates `Cargo.toml`.
2. **AI Agents as First-Class CLI Consumers:** Modern CLI tools are routinely driven by AI coding agents (Claude Code, Arena, Cursor, Codex). Agents require structured JSON execution contracts (`--json`), machine-readable CLI schema introspection (`mycli schema`), automated Agent Skill documents (`.agents/skills/<name>/SKILL.md`), and Model Context Protocol (MCP) server endpoints (`mycli mcp`).
3. **Strict Zero-Cost Feature Boundaries:** Not every CLI needs templating, JSON-RPC, or shell completions. In accordance with Decision 3 and Decision 5, auxiliary capabilities must be strictly feature-gated behind opt-in Cargo feature flags, preserving zero extra dependencies and zero binary size impact for basic CLIs.
4. **Pluggable Architecture:** Skill generation must be an open trait (`SkillRenderer`), allowing developers to bring their own formatting or template engines (Tera, Askama, Tinytemplate) while providing an optional, battery-included MiniJinja implementation (`skill-minijinja`).

---

## Decision

### 1. Retire the `build` API and Directory Scanning
Deprecate and remove `clifacet::build::commands()` and the `build` Cargo feature. Declarative controllers (`#[clifacet::app]` and `#[clifacet::commands]`) serve as the canonical command registration mechanism, removing the need for `build.rs` scripts and compile-time filesystem scanning.

### 2. Machine-Readable Execution Contract (`--json`)
Support an opt-in `--json` global flag. When passed:
- Successful command executions serialize the returned data structure (`T: Serialize`) as formatted JSON on `stdout`.
- Command failures serialize the `Failure` into a standardized JSON error schema on `stderr` with non-zero exit code:
  ```json
  {
    "status": "error",
    "error": {
      "kind": "validation",
      "code": 2,
      "message": "Required argument <service> is missing"
    }
  }
  ```

### 3. Machine-Readable CLI Schema Introspection (`schema`)
Behind the `schema` Cargo feature, provide `mycli schema` (or `--schema`). Reflects the complete CLI hierarchy, command names, argument types, defaults, required status, environment variable bindings, and docstrings into an introspectable JSON Schema specification for automated agents and documentation tools.

### 4. Pluggable `SkillRenderer` Trait (`skill`)
Behind the `skill` Cargo feature, define the zero-dependency `SkillRenderer` trait:
```rust
pub trait SkillRenderer {
    fn render(&self, cli: &CliMetadata) -> Result<String, Failure>;
}
```
Any user can implement this trait to format CLI metadata into custom documentation or skill formats without depending on third-party templating engines.

### 5. Built-in MiniJinja Skill Renderer (`skill-minijinja`)
Behind the `skill-minijinja` Cargo feature:
- Depend on `minijinja` (lightweight, zero heavy dependencies).
- Embed a default Jinja2 template (`templates/skill.md.j2`) producing lint-clean `.agents/skills/<name>/SKILL.md` agent documentation.
- Provide built-in `write-skill` CLI command supporting `--stdout`, `-o <path>`, and `--check` (fails in CI if committed skill file has drifted from code).
- Allow user overrides via custom `.j2` template files or attributes.

### 6. Model Context Protocol Server (`mcp`)
Behind the `mcp` Cargo feature, inject `mycli mcp`. Runs an MCP (Model Context Protocol) JSON-RPC tool server over `stdio`. Every command authored in `#[clifacet::commands]` is automatically exposed as an MCP Tool with input schemas derived from argument types, allowing instant integration into AI agent hosts.

### 7. Context-Aware Dynamic Shell Completions (`completions`)
Behind the `completions` Cargo feature, inject `mycli completions <shell>` supporting Bash, Zsh, Fish, and PowerShell. Support dynamic argument completion hooks where completion callbacks have access to `&self.cx` to query live databases or services.

---

## Consequences

- Completely eliminates `build.rs` dependencies and compile-time environment protocols.
- Enables CLIs to seamlessly act as human terminal utilities, scriptable Unix pipe tools, and AI agent endpoints.
- Preserves ultra-lean base builds: `completions`, `schema`, `mcp`, `skill`, and `skill-minijinja` are completely independent, opt-in feature flags.
- Prevents documentation drift: `write-skill --check` can be enforced in CI alongside `prose-check` and `check-docs`.
- Establishes Milestones M-19 and M-20 with tasks CLI-71 through CLI-78.
