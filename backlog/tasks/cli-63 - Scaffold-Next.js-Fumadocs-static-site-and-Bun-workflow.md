---
id: CLI-63
title: Scaffold Next.js Fumadocs static site and Bun workflow
status: To Do
assignee: []
created_date: '2026-11-19 10:21'
labels: []
milestone: m-17
dependencies: []
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Setup the Next.js and Fumadocs application inside `docs/` adhering to the Archont561 standard: `package.json` with Fumadocs dependencies (`fumadocs-core`, `fumadocs-mdx`, `fumadocs-ui`, `next`, `react`, `tailwindcss`), `next.config.mjs` exporting static files (`output: 'export'`) with GitHub Pages subpath handling, `source.config.ts`, `tsconfig.json`, `postcss.config.mjs`, `src/app/`, and `src/lib/source.ts`. Add Pixi tasks (`docs-dev`, `docs-build`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/package.json declares Fumadocs dependencies and locks cleanly with bun.lock
- [ ] #2 docs/next.config.mjs configures static export (output: 'export') and basePath for production (/${repository})
- [ ] #3 Fumadocs source loader (docs/src/lib/source.ts) and root docs layout/page render the documentation tree
- [ ] #4 Pixi task docs-build executes bun run build in docs/ and produces verified static assets in docs/out
<!-- AC:END -->
