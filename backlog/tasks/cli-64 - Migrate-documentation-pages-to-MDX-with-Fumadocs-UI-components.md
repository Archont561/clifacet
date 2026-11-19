---
id: CLI-64
title: Migrate documentation pages to MDX with Fumadocs UI components
status: Done
assignee: []
created_date: '2026-11-19 10:21'
labels: []
milestone: m-17
dependencies:
  - CLI-63
ordinal: 64000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Migrate the repository documentation (`guides/`, `concepts/`, `reference/`, `architecture/`, `development/`) to MDX under `docs/content/docs/` with `meta.json` navigation files. Integrate Fumadocs UI components (`<Callout>`, `<Cards>`, `<Card>`, `<Tabs>`, `<Tab>`, `<Steps>`, `<Step>`, `<Accordions>`, `<Accordion>`) across guides and reference pages (e.g. installation/quickstart steps, terminal mode comparisons, derive attribute options). Configure `docs/mdx-components.tsx` with default Fumadocs UI components.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All user and developer guides, reference documents, and architectural overviews are converted to .mdx under docs/content/docs/
- [x] #2 meta.json files define section hierarchy and navigation ordering for all documentation directories
- [x] #3 User guides leverage interactive Fumadocs UI components including <Tabs> for package managers, <Steps> for onboarding, and <Callout> for operational invariants
- [x] #4 docs/mdx-components.tsx exports useMDXComponents with fumadocs-ui/mdx defaults and custom components
<!-- AC:END -->
