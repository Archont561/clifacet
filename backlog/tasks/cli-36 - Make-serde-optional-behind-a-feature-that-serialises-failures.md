---
id: CLI-36
title: Make serde optional behind a feature that serialises failures
status: Done
assignee: []
created_date: '2026-11-01 22:46'
updated_date: '2026-11-06 09:05'
labels: []
milestone: m-9
dependencies: []
type: task
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
serde is a non-optional dependency of clifacet that the crate never references, while serde_json sits behind a json feature with no cfg site and no flag that reaches it. Turn the first into the second: one serde feature that gives Failure and FailureKind a Serialize impl, redacts Secret, and is what a JSON failure body is built from.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 serde is optional: `cargo tree -p clifacet --no-default-features` contains no serde, and the default build is unchanged in behaviour.
- [x] #2 A `serde` feature gives `Failure` and `FailureKind` a `Serialize` impl whose body is stable enough to document: a kind and a message.
- [x] #3 `Secret` serialises redacted, with a test proving the value cannot appear in the output.
- [x] #4 The `json` feature is either rebuilt on that impl with a real `cfg` site and a flag that reaches it, or retired with a written reason.
- [x] #5 The feature list in the documentation states what each feature adds and what it costs to enable.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-11-04: implemented test-first. Red: `crates/clifacet/tests/serialization.rs` (gated on `feature = "serde"`, which the gates build with `--all-features`) was written against the newly declared feature and failed to compile with the three missing `Serialize` bounds (`Failure`, `FailureKind`, `Secret<String>`). Green: 13 tests.
- **AC#1.** `serde` is `optional = true` behind `serde = ["dep:serde"]`. `cargo tree -p clifacet --no-default-features -e normal` and the default build both contain no serde. With `--features serde`, the tree gains `serde`, `serde_core` and `serde_derive` and nothing else. `Cargo.lock` is unchanged: every cargo command ran `--locked`, and `serde_json` moved from an optional to a dev-dependency, so its edge stays recorded. The default build is unchanged in behaviour: serde was linked but never referenced.
- **AC#2.** `#[cfg_attr(feature = "serde", derive(serde::Serialize))]` on `Failure` gives `{"kind":"validation","message":"…"}`, kind first. `FailureKind` serialises as its lowercase name. Both are documented on the types and in the feature table; a test pins the exact string, including field order.
- **AC#3.** `Secret<T>` serialises as `"[redacted]"` for any `T`, without asking `T` to serialise. Tests cover a token, the empty string, a value full of JSON quotes and escapes, non-ASCII, and the marker itself, each producing exactly the marker. A secret embedded in a larger JSON value never puts the credential in the output. There is deliberately no `Deserialize`, and the type's docs now say so.
- **AC#4: `json` retired.** Written reason: decision-3 makes output and exit mapping application-owned, so a library flag selecting JSON would contradict an accepted decision. With `serde`, a JSON failure body is `serde_json::to_string(&failure)` in the application, under its own flag and on its own stream, so a `json` feature would only wrap that one call and add a dependency. The `UNUSED_FEATURES` entry in `scripts/check_features.py` is deleted and the table is now empty. The matrix row is `serde`, with a real `cfg` site, and `check-features` passes every combination. This is a breaking change for anyone who enabled `json` in 0.2.0, which is acceptable at 0.x; the commit is marked `!`.
- **AC#5.** `docs/getting-started.md#features` has a table of what each feature adds and what it costs, named by the crates it pulls in rather than by counts that would rot; README, development.md, command-core.md and the crate docs link to it. Measured on Linux from the lock: `interactive` +28 crates, `tui` +54, `serde` +3.
- **BDD narrowing, stated.** No scenario. The library writes no JSON (decision-3), so there is no user-visible workflow in the crate to describe: the promise is the serialised *shape*, which is an API property proven by the rstest tables above. A scenario would only restate them. The existing `secret.feature` scenarios cover redaction in diagnostics and provenance, not serialisation; that half is proven only by the tests above.
- `test-local` is green. All ACs are checked; the task stays In Progress until the maintainer accepts it.

- 2026-11-06: merged in PR #47 (`ca0865de`), as `d644a0e9` (serde feature) and `80e330e9` (`json` retired, breaking). All five ACs are checked; closed with maintainer approval.
<!-- SECTION:NOTES:END -->
