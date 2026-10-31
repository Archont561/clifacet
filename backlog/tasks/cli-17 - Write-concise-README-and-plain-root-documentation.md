---
id: CLI-17
title: Write concise README and plain root documentation
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-5
dependencies:
  - CLI-13
  - CLI-14
  - CLI-15
type: task
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Document shipped behavior without a separate site or invented API.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Root README is concise and links to guides under root docs/.
- [x] #2 Getting started, modes, derive, TUI composition, testing and contributing cover the implemented API.
- [x] #3 Documentation examples and links are checked in CI; domain examples are neutral.
<!-- AC:END -->

- 2026-10-31: the documentation had one reference page and no way in. `docs/README.md`
  still said the project was "at the planning stage", the README was a develop-command
  list, and nothing checked whether any of it was true.
- AC#1: the root README is now a map rather than a manual — pitch, one compiling command
  example, three runnable invocations, a crate table, and a guide table that links into
  `docs/`. It stays under 100 lines so the guides carry the detail.
- AC#2: five guides cover the implemented API, each extracted from the code rather than
  from intention — [getting started](docs/getting-started.md) (add the crate, write a
  command, run it), [modes](docs/modes.md) (the three-input policy, the four modes, the
  stream and exit-code tables), [derive](docs/derive.md) (the attribute contract, the two
  ways to build input, what the derive deliberately does not do),
  [TUI composition](docs/tui-composition.md) (the public components, the generated form,
  both escape hatches, headless testing), [contributing](docs/contributing.md) (bootstrap,
  task surface, the TDD/BDD requirement, hooks, where things live). `docs/README.md` is an
  index again, and `docs/command-core.md` stays the reference it already was.
- AC#3: `scripts/check_docs.py` + the `check-docs` task make the guides part of the
  build, and this is the part worth reading twice. Links and `#fragment` anchors must
  resolve. Every ```` ```rust ```` fence is copied into
  `crates/clifacet-example/tests/documented_snippets.rs`, which the suite compiles and
  runs — 8 snippets today, and the file is committed so CI fails when a page was edited
  without regenerating it. `rust,ignore` is the escape hatch for a deliberate fragment,
  which is exactly what the reference page's shapes are (all seven of its fences were
  retagged). Every `clifacet::…` path is checked *segment by segment* against what the
  crates declare, stopping where a crate re-exports someone else's module
  (`clifacet::tui::ratatui::…` is ratatui's). Neutrality is a check too: a short list of
  words the reference example is deliberately not fails the build if a guide uses one.
- TDD evidence: `scripts/tests/test_check_docs.py` (19) was written first and failed with
  `ModuleNotFoundError`; the snippet half was then proven the hard way — the generated
  file did not exist, `cargo test` picked it up, and it compiles as part of the suite.
  Four implementation mistakes were found by the tests and fixed in the code, not the
  test: `check()` conflated "findings about pages" with staleness of a generated file, so
  a fixture tree could not be checked without creating that file; checking only the last
  path segment let `clifacet::policy::NoSuchThing::new()` pass on the strength of `new`
  existing somewhere else; `checkout` was in the domain list and matched Git's own word
  in `development.md`; and the first generated file was rewritten by `cargo fmt`, so the
  staleness check failed on a file nobody had touched. That last one is now structural:
  every generated test carries `#[rustfmt::skip]`, because a verbatim copy of a page is
  not a file for the formatter to reflow. (`rustfmt.toml`'s `ignore` would have been the
  obvious fix and is nightly-only.)
- Design decisions: the snippet file lives in `clifacet-example` rather than in the
  library, because the example crate is the consumer — a snippet that compiles there
  compiles for a caller, and generating in `clifacet` would need a dependency on the
  example crate to reach `clifacet_example::…` paths. Checking whole paths rather than
  final segments costs a re-export boundary rule but catches the failure mode that
  matters, an invented type. The domain-word list is deliberately tiny and concrete
  instead of a heuristic: a check nobody can predict is a check people route around.
- Coverage: unchanged at 936/941 (99.47%) — no library line changed. The suites grew to
  24 + 229 Rust tests (8 documented snippets) and 143 Python tests (was 124), and
  `check-docs` is wired into `test-local`, `quality`, the CI quality job and the
  pre-commit hook, with an Alint rule for the script.

