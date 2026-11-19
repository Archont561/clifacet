---
id: CLI-65
title: 'Adapt snippet verification, link checking, and prose linting for MDX'
status: Done
assignee: []
created_date: '2026-11-19 10:22'
labels: []
milestone: m-17
dependencies:
  - CLI-64
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Update documentation validation tooling (`tools/_lib/check_docs.py`, `.alint.yml`, and `vale.toml`) to recognize `.mdx` files. Ensure Rust code fences inside `.mdx` are extracted into `examples/simple/tests/documented_snippets.rs`, relative links/anchors resolve against Fumadocs slug routes, and prose linting validates MDX content without parsing errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tools/_lib/check_docs.py parses .mdx files across docs/content/docs/ and verifies headings, relative links, and API symbols
- [x] #2 Rust fences in .mdx files are extracted and compiled as tests in documented_snippets.rs
- [x] #3 Vale prose linter and Markdown/MDX linters pass on all .mdx documentation pages
- [x] #4 Repository checks (alint, check-docs, prose-check) pass cleanly in CI and local gates
<!-- AC:END -->
