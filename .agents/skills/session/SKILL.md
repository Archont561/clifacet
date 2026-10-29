---
name: session
description: Reconstruct the project state at the start of a session or whenever chat history is missing or incomplete.
---

# Session

Use before planning or editing when the chat has no useful history, a session
has just begun, or context was lost or condensed beyond what the task needs.

1. Read `AGENTS.md`, the current request and this skill. Treat workspace files
   and the actual Git state as evidence, not memory or assumptions.
2. Inspect the relevant files and, in a Git checkout, `git status --short`
   and `git branch --show-current`. Preserve pre-existing changes. Do not fetch,
   switch branches, commit, push or release without authorization.
3. Before checks or tests, follow the Pixi restore rule in `AGENTS.md`. If
   usable, inspect relevant tasks and milestones with `pixi run --frozen
   --offline backlog-offline -- milestone list` and `pixi run --frozen --offline
   backlog-offline -- task list`; read relevant decisions and docs. These
   offline tasks fail if the locked Bun packages are unavailable. In that case,
   read the committed backlog directly and report the limitation; never invent
   the missing history or silently download dependencies.
4. State only material findings and proceed with a clear request. If the goal
   or safe next action remains unclear, consult `regain-direction` and ask one
   focused question. Do not impose a sprint or change the requested scope.
5. At handoff, record what changed, what was verified, what remains open, and
   any blockers. Do not claim unrun checks passed.
