---
id: decision-7
title: Crates publication through trusted release workflow
date: '2026-10-29 15:28'
status: accepted
---
## Context
The maintainer reports that crates.io trusted publishing is enabled for Archont561/clifacet, release.yml, GitHub environment `crates`. The `clifacet` and `clifacet-macros` names exist on crates.io at 0.0.0.

## Decision
Use release.yml and environment `crates` with job-scoped OIDC permission and short-lived publishing credentials. Publish the macros crate first; wait for its version to be visible in the registry/index; then publish the main crate. Require explicit release authorization and use a fail-closed, resumable process.

## Consequences
Verify the registry trust configuration, pin official actions and rehearse safely. Author `release.yml` in this repository against the documented cargo trusted-publishing flow, with pinned official actions; do not depend on a generator whose crates.io support is unverified. A failed second publish is a partial release that must be detectable and recoverable, not re-uploaded blindly.

## Implementation note

2026-10-30: the maintainer re-confirmed the registry binding — trusted publishing is enabled for both `clifacet` and `clifacet-macros` against `release.yml`, with GitHub environment `crates`. `release.yml` is authored (CLI-4); the default dispatch is a dry run. A live publish needs the `crates` environment approval plus an unchecked `dry_run`, and has not happened.
