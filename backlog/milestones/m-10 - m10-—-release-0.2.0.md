---
id: m-10
title: "M10 — Release 0.2.0"
---

## Description

The second release, after the authoring ergonomics and the declarative surface. The version-bump runbook is exercised end to end for the first time: the remaining acceptance criterion of CLI-20 is that bump-version 0.1.1 runs from a main dispatch with --relock.

Exit: every gate in the 0.1.0 release month passes again, the runbook does its own relock without a hand-edited lockfile, both crates publish from one authorized dispatch, and the changelog and release notes come from the commits.
