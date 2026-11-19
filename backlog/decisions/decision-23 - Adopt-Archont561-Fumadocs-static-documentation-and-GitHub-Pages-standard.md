---
id: decision-23
title: Adopt Archont561 Fumadocs static documentation and GitHub Pages standard
date: '2026-11-19 10:22'
status: accepted
---
## Context

In Decision 8, documentation was kept deliberately as plain repository Markdown files in root `docs/` with snippet compilation tests and no separate documentation generator for the initial 0.1.0 release.

As `clifacet` evolved through 0.7.0 into a mature poly-modal CLI framework with multiple terminal frontends (noninteractive, inquire questionnaires, full-screen Ratatui TUI forms), hierarchical Figment configuration presets, and command registration macros, plain Markdown without rich component layout presents ergonomic friction:
1. Multi-modal options, derive attribute parameters, and installation choices benefit from interactive UI components (such as `<Tabs>`, `<Steps>`, `<Callout>`, `<Cards>`, and `<Accordions>`).
2. The `Archont561/*` ecosystem standardizes documentation on Next.js + Fumadocs static site export (`output: "export"`) deployed via GitHub Pages (`.github/workflows/pages.yml`) using Bun.
3. Documentation should be deployed automatically upon push to `main` without requiring manual build artifacts in source git history.

## Decision

Adopt the `Archont561/*` documentation standard:
1. **Fumadocs Static Site:** Configure a Next.js static site inside `docs/` using Fumadocs (`fumadocs-core`, `fumadocs-mdx`, `fumadocs-ui`), Tailwind CSS, and Bun lockfile.
2. **MDX Documentation with Fumadocs UI:** Structure documentation under `docs/content/docs/` as `.mdx` files accompanied by `meta.json` navigation manifests. Leverage Fumadocs UI components (`<Callout>`, `<Cards>`, `<Card>`, `<Tabs>`, `<Tab>`, `<Steps>`, `<Step>`, `<Accordions>`, `<Accordion>`) across guides, concepts, reference, architecture, and development docs.
3. **Static Export for GitHub Pages:** Configure Next.js with `output: "export"` and production `basePath: "/clifacet"`.
4. **Automated Publishing Workflow:** Add `.github/workflows/pages.yml` with preflight detection querying GitHub Pages REST API, Bun-based static build, Pages artifact upload, and deployment to the `github-pages` environment.
5. **Quality Gates Preservation:** Adapt repository verification tools (`check_docs.py`, Vale prose linting, snippet compilation, and alint) to parse `.mdx` files, validate routes, and test embedded Rust code snippets.

## Consequences

- Supersedes Decision 8's plain Markdown constraint.
- User guides and reference pages gain rich, interactive UI components while remaining readable in source form.
- Documentation builds and deploys automatically to `https://archont561.github.io/clifacet/` via GitHub Actions when enabled.
- `tools/_lib/check_docs.py` and prose linting are expanded to validate `.mdx` files.
