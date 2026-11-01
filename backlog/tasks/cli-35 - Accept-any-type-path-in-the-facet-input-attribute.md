---
id: CLI-35
title: Accept any type path in the facet input attribute
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
labels: []
milestone: m-7
dependencies: []
type: task
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The facet input attribute only forwards a token tree that happens to name a struct path; a type with generics, a qualified path or an alias does not survive the round trip. The macro should accept a syn type and either use it or reject it with a message naming the limitation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 A qualified path (`crate::input::MyInput`), a generic type (`MyInput<'a>`), and an associated type (`Self::Input`) are all accepted where the input attribute names a type.
- [ ] #2 A form that cannot be supported is rejected with the error span on the offending tokens, not on the whole attribute.
- [ ] #3 Each accepted form has a passing test and each rejected one has a trybuild case with the exact message asserted.
- [ ] #4 The documented attribute keys describe what the input value may be, and the documentation snippet compiles.
<!-- AC:END -->
