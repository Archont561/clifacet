---
id: m-15
title: "M15 — Hierarchical configuration presets and diagnostic citations"
---

## Description

Phase 2 of decision-22: extending Figment configuration with multi-source cascading presets (system-wide `/etc/`, per-user `~/.config/`, and local project directory), profile switching helpers (`FIGMENT_PROFILE=production`), context injection ergonomics (`cx: &Figment`), and line-level diagnostic citations in `--dry-run` and error reports.

Exit: applications can configure cascading configuration file paths with one builder call; environment profiles switch configurations cleanly; dry-run provenance and error diagnostics cite the exact file, section, and line number where a value was defined.
