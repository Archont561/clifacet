---
id: decision-9
title: Agent skills and regain-direction support
date: '2026-10-29 15:28'
status: accepted
---
## Context
The maintainer wants agent skills and a local skill for recovering direction mid-session.

## Decision
Use repository agent skills for session, backlog, TDD, BDD, refactor, audit and relevant planning activities. Add a local `regain-direction` skill that pauses, inspects evidence, summarizes goal/done/open/drift/unknowns, proposes up to three next steps, and asks the maintainer before scope changes.

## Consequences
Select and document a consistent source and lock convention; validate committed skills and locks in CI. Do not silently discard, commit, push, close tasks or release. The existing skills/lock conventions differ across repositories, so reuse is not assumed to be drop-in.
