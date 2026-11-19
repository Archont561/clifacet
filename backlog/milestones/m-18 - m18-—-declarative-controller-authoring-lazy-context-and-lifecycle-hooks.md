---
id: m-18
title: "M18 — Declarative controller authoring, lazy context and lifecycle hooks"
---

## Description

Adopt a class-like, Pydantic/Typer-inspired declarative authoring experience (decision-24): a root application struct declaring global options and auto-wired lazy context, command methods authored on `impl` blocks with direct `&self` access, lifecycle execution hooks (`before_run`, `after_success`, `after_failure`), and declarative nested subcommand groups with context extraction.
