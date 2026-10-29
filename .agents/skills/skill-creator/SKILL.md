---
name: skill-creator
description: Create or revise a focused local agent skill with clear activation and safety boundaries.
---

# Create a skill

Confirm the recurring problem and when the skill should activate. Create a
folder under `.agents/skills/` with `SKILL.md`, YAML frontmatter (`name` matching
the folder and a meaningful `description`), concise steps and explicit stop
conditions. Link only to existing references. Keep repository-local skills
committed and reviewable; do not silently overwrite them via remote restore.
Run `pixi run --frozen skills-check` and its tests.
