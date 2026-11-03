---
id: CLI-37
title: Prove every declared Cargo feature has a cfg site
status: Done
assignee: []
created_date: '2026-11-01 22:46'
updated_date: '2026-11-03 07:58'
labels: []
milestone: m-9
dependencies: []
type: task
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A declared feature that no cfg mentions is a promise the crate does not keep. Extend the feature-boundary script so the matrix rows stay derived from the manifest and a feature without a cfg site fails, then retire or implement the json feature on that evidence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The feature script fails when a declared feature has no `cfg(feature = "…")` site, and its own test covers that failure.
- [x] #2 Matrix rows are still derived from the manifest, and `--print-matrix` keeps its shape.
- [x] #3 A feature without a `cfg` site is not allowlisted silently: any exception carries a written reason in the script.
- [x] #4 The guard runs in CI, in the same job as the existing feature-boundary check.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-11-03: AC#1 to AC#4 landed. `scripts/check_features.py` gained `cfg_sites`, which strips comments and scans the crate's `src` for `cfg(feature = "…")`, `cfg!(feature = …)` and `cfg_attr(…, feature = …)` (nested `cfg(all(..))` included); `unused_features` reports a declared feature nothing mentions; `main` runs that check before any `cargo check`, because a feature that gates nothing is a manifest mistake and finding it after four compiles wastes the slowest part. A guide that writes `cfg(feature = "tui")` in prose does not count as a site, and a fixture crate is never granted the repository's own allowlist.
- Exceptions are data and cost a sentence: `UNUSED_FEATURES` is a mapping from feature to written reason, `expect_unused_features` refuses a blank one, and `test_the_declared_exceptions_still_describe_the_crate` fails when an exception outlives its feature. Today the table holds exactly one entry — `json` — pointing at CLI-36, so AC#3's "not allowlisted silently" is enforced rather than stated.
- Evidence the guard bites: with the `json` entry temporarily removed, `check_features.py --check-cfg-sites` exits 1 naming `json`; restored, it reports 'every declared feature gates some code (3 checked)'. `--print-matrix` is unchanged (`no-default-features`, `default`, `interactive`, `json`, `tui`, `all-features`), and the full matrix passes: 5 distinct configurations, 'every supported combination compiles'.
- AC#4 needed no workflow change: the guard rides the existing `check-features` step, which `ci.yml` already runs in the repository-quality job (and `prove_msrv.py` reuses the same module, so the MSRV matrix is guarded too).
- TDD: the ten new tests were written first and collected red — `ImportError: cannot import name 'cfg_sites'` — before the implementation existed. Three test-side mistakes were fixed on the evidence rather than by weakening the product: `unused_features` was wrongly given fixture dicts instead of manifests, an exception default inside the predicate confused 'fact' with 'policy' (split into `unused_features` and `expect_unused_features`), and the allowlist was being granted to a temporary fixture crate by filename. 26 tests in this module, 315 Python tests overall, 302 Rust tests.

- 2026-11-03: CI is green on the change — PR #42, run [41599242677](https://github.com/Archont561/clifacet/actions/runs/41599242677) passes all seven checks: Repository and Rust quality (which hosts `check-features`, so AC#4 is proved in CI and not only locally), Tests and source coverage, Minimum supported Rust version (the same `check_features` matrix at 1.88), Supported platforms on ubuntu, macos and windows, and Conventional commit messages.
<!-- SECTION:NOTES:END -->
