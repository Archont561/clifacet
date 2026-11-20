---
id: CLI-75
title: Implement MiniJinja skill renderer and embedded template
status: Done
assignee: []
created_date: '2026-11-19 14:30'
labels: []
milestone: m-20
dependencies:
  - CLI-74
ordinal: 75000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the built-in MiniJinja skill renderer behind the `skill-minijinja` Cargo feature (decision-25). Embed a default Jinja2 template (`templates/skill.md.j2`) producing markdown lint-clean `.agents/skills/<name>/SKILL.md` documents with YAML frontmatter, overview, non-interactive execution rules, global options, and per-command documentation. Support custom template string overrides via `#[clifacet::app(skill(template = "..."))]`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 skill-minijinja feature compiles minijinja and embeds default templates/skill.md.j2
- [x] #2 Rendered output strictly conforms to markdownlint and Vale formatting rules without stray whitespace
- [x] #3 Custom .j2 template overrides are supported via app attribute or command flag
- [x] #4 Integration tests verify generated skill markdown across varied command shapes and options
<!-- AC:END -->
