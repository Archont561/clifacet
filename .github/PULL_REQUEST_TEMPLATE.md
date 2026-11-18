# Pull Request

## Summary

- What changed and why?
- Related issue or `backlog/` task ID:

## Affected Surfaces

- [ ] `crates/clifacet` (core library, settings, runners, TUI, prompts)
- [ ] `crates/clifacet-macros` (command derive, proc-macros)
- [ ] `examples/simple`
- [ ] `examples/discovered`
- [ ] `examples/backlog-lite`
- [ ] Documentation (`docs/`, `README.md`, `AGENTS.md`)
- [ ] Tooling, CI, policy, or dependencies (`scripts/`, `tools/`, `.alint.yml`, `.github/`)

## Verification Checklist

Run checks through Pixi before requesting review:

- [ ] `pixi run fmt-check` (cargo fmt --all -- --check)
- [ ] `pixi run lint` (cargo clippy --workspace --all-targets --all-features -- -D warnings)
- [ ] `pixi run alint` (alint repository and hygiene policy)
- [ ] `pixi run check-docs` (link integrity, anchors, documented item names, snippet compilation)
- [ ] `pixi run prose-check` (Vale prose style validation)
- [ ] `pixi run test` (nextest workspace unit and integration tests)
- [ ] `pixi run test-doc` (cargo test doc tests)
- [ ] `pixi run test-scripts` (pytest script tests)
- [ ] `pixi run deny-offline` (offline cargo-deny bans, licenses, sources)
- [ ] `pixi run check-features` (feature combination compile matrix)
- [ ] `pixi run actionlint` (GitHub Actions workflow validation with shellcheck)
- [ ] `pixi run sandbox-check` (pixi-sandbox configuration and generated workflows)
- [ ] `pixi run skills-check` (agent skills frontmatter validation)
- [ ] `pixi run backlog-check` (backlog task cards status consistency)

## Architecture Guardrails

- [ ] Command and result seams remain deterministic and capability-driven.
- [ ] No secrets leaked in errors, debug displays, serialization, or `--help`.
- [ ] Every commit message follows Conventional Commits as configured in `.versionrc`.
