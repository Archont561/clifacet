---
id: m-8
title: "M8 — Plugins as binaries on the path"
---

## Description

A documented recipe, not a library feature: decision-17 was rejected as an API on 2026-11-06. An application exposes a <cli>-<name> binary as <cli> <name>, the way git-lfs becomes git lfs, with clap and std alone, following the behaviour decision-17 measured. CLI-29 carries it.

Exit: the authoring guide's plugin section compiles under check-docs, states the forwarding rules and the Windows differences, refuses plugins on a headless plan, and clifacet itself gains no plugins module, feature or dependency.
