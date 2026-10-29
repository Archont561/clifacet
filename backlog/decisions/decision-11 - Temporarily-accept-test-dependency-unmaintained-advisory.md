---
id: decision-11
title: Temporarily accept test dependency unmaintained advisory
date: '2026-10-29 15:46'
status: accepted
---
## Context
`rstest-bdd` 0.6.0 depends on `basic-toml` through i18n-embed and i18n-config. RustSec reports `RUSTSEC-2026-0374` as an unmaintained-crate advisory. The maintainer selected retaining rstest-bdd with a narrowly documented exception rather than switching BDD frameworks or deferring it.

## Decision
Temporarily ignore exactly `RUSTSEC-2026-0374` in both cargo-deny and cargo-audit; keep all other deny and audit warning gates active. This is currently a test dependency, not a disclosed vulnerability in the published runtime.

## Consequences
The ignore is ID-wide, not scoped to the current dependency path. Review the graph on dependency updates and remove the exception once a maintained upstream alternative is available. The exception must remain visible in developer documentation and release review.
