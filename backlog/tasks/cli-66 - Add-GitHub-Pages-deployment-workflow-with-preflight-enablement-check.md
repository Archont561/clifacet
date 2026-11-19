---
id: CLI-66
title: Add GitHub Pages deployment workflow with preflight enablement check
status: Done
assignee: []
created_date: '2026-11-19 10:22'
labels: []
milestone: m-17
dependencies:
  - CLI-63
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create `.github/workflows/pages.yml` adhering to the Archont561 standard. Implement GitHub Pages preflight detection using the GitHub REST API (cleanly skipping deployment without error when Pages is not enabled or not set to GitHub Actions source), build static export using Bun, upload artifact (`actions/upload-pages-artifact` for `docs/out`), and deploy (`actions/deploy-pages` to `github-pages`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 .github/workflows/pages.yml triggers on push to main (for docs/** and pages.yml) and manual workflow_dispatch
- [x] #2 Preflight job queries GitHub Pages API (/repos/${GITHUB_REPOSITORY}/pages) and cleanly reports enablement status in GITHUB_STEP_SUMMARY
- [x] #3 Build job installs dependencies via Bun, runs bun run build in docs/, and uploads docs/out via actions/upload-pages-artifact
- [x] #4 Deploy job targets github-pages environment and deploys via actions/deploy-pages
<!-- AC:END -->
