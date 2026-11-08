---
id: m-12
title: "M12 — Runner, context and registered commands"
---

## Description

The first sprint of decision-19, released as 0.4.0. An application gets a runner that owns the pipeline (parse, ask, validate, build the context, run, render, exit), commands that receive the application's dependencies as a context instead of globals, and a one-line-per-command `clifacet::commands!(..)` registration. `examples/backlog-lite` moves onto all three as the proof.

Exit: backlog-lite's `main.rs` is one call, adding a command touches one place, the `CLIFACET_BACKLOG_BIN` variable and the test `thread_local!` are gone, every existing test passes, and 0.4.0 publishes.
