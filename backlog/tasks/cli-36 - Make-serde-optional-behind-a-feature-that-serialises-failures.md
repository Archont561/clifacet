---
id: CLI-36
title: Make serde optional behind a feature that serialises failures
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
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
- [ ] #1 serde is optional: `cargo tree -p clifacet --no-default-features` contains no serde, and the default build is unchanged in behaviour.
- [ ] #2 A `serde` feature gives `Failure` and `FailureKind` a `Serialize` impl whose body is stable enough to document: a kind and a message.
- [ ] #3 `Secret` serialises redacted, with a test proving the value cannot appear in the output.
- [ ] #4 The `json` feature is either rebuilt on that impl with a real `cfg` site and a flag that reaches it, or retired with a written reason.
- [ ] #5 The feature list in the documentation states what each feature adds and what it costs to enable.
<!-- AC:END -->
