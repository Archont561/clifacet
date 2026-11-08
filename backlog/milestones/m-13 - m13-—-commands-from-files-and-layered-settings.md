---
id: m-13
title: "M13 — Commands from files and layered settings"
---

## Description

The second sprint of decision-19, released as 0.5.0: `clifacet::build` discovers `src/commands/` from the application's build script, `_name.rs` files are private helper modules, and settings layer as flag > environment > file > default with provenance and an injectable environment. Functions as commands follow once both exist.

Exit: dropping a file into `src/commands/` adds a command with no other edit and every error points at the application's own file; a config file, `--config`, `--no-config` and the environment are honoured in that order, tested without mutating the process environment; 0.5.0 publishes.
