---
id: m-20
title: "M20 — Pluggable agent skills, MiniJinja templates, and MCP server"
---

## Description

Implement AI agent integration primitives (decision-25): pluggable `SkillRenderer` trait, built-in MiniJinja skill generator (`skill-minijinja`) rendering `.agents/skills/<name>/SKILL.md` with drift checking (`write-skill --check`), Model Context Protocol stdio server (`mycli mcp`), and dynamic shell completions.
