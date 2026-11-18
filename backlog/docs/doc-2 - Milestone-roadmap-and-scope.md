---
id: doc-2
title: Milestone roadmap and scope
type: other
created_date: '2026-10-29 15:28'
updated_date: '2026-11-18 21:35'
---
# Roadmap

The canonical milestone files describe exits; tasks are assigned to their relevant milestone. All items are unimplemented unless a task and evidence say otherwise.

1. **m-0 Foundation:** environment, repository gates, sandbox and release-workflow feasibility.
2. **m-1 Clap-first core:** one domain-neutral, non-interactive vertical slice.
3. **m-2 Interactive:** policy and prompts with no forbidden prompter construction.
4. **m-3 TUI:** public ratatui components, generated form and overrides.
5. **m-4 Parity and safety:** public cross-mode harness, output and secret guarantees.
6. **m-5 0.1.0:** qualification, real plain docs and authorized release readiness.
7. **m-6 Authoring ergonomics:** choice, secret and repeated questions, a classified confirmation, provenance from the parse, a plan seam, parity sugar and one failure renderer — decision-18, taken from the backlog.md wrapper.
8. **m-7 Declarative command surface:** an argument declared once as an attributed struct field (decision-13, done), a nested tree (decision-14, CLI-32), and — deferred until the tree is measured — one command per file under one macro-layering rule (decisions 15 and 16, CLI-33).
9. **m-8 Plugins as binaries on the path:** a documented recipe, not a library feature — decision-17 was rejected as an API; CLI-29 carries the recipe.
10. **m-9 Feature boundaries and machine-readable output:** every declared feature proved to have a `cfg` site, `serde` made optional behind a feature that serialises failures with secrets redacted, and the `json` feature either implemented on that evidence or retired.
11. **m-10 Release 0.2.0:** the version-bump runbook exercised end to end on a real bump, every 0.1.0 gate passing again, and both crates published from one authorized dispatch.
12. **m-11 Release 0.3.1:** secret hiding from `--help` (CLI-43) and dependency on clap without its default features (CLI-44).
13. **m-12 Runner, context and registered commands:** `clifacet::App` runner owning the pipeline, context dependency injection for commands (`Handler<Cx>`), and `clifacet::commands!` tree registration, released as 0.4.0.
14. **m-13 Commands from files and layered settings:** build-script command discovery (`clifacet::build`), one-attribute `#[clifacet::registry]`, layered settings (`flag > env > file > default`), and functions as commands, released as 0.5.0.
15. **m-14 Figment configuration and canonical automation:** Figment-backed layered configuration engine (CLI-59, decision-22), command function mapping, Serde derivation on generated inputs, key normalization, and migration of repository automation to canonical Tasknest tooling, released as 0.6.0.
16. **m-15 Hierarchical configuration presets and diagnostic citations:** multi-source cascading loaders (`/etc/`, `~/.config/`, local directory), environment profile switching, context injection ergonomics, and line-level file provenance and error citations (decision-22 Phase 2).
17. **m-16 Shell completions, schemas and TUI component expansion:** shell completion generation via `clap_complete` derived from registered command functions, and expanding Ratatui form components with choice selectors, toggles, and multi-value collectors.

Each stage includes its own behavior and safety tests; m-4 is completion and publication of the shared harness, not permission to defer testing. Milestones are planning boundaries, not guaranteed schedules.
