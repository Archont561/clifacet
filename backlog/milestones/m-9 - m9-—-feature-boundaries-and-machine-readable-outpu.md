---
id: m-9
title: "M9 — Feature boundaries and machine-readable output"
---

## Description

Make the declared feature set mean something. Today serde is a non-optional dependency nothing uses, the json feature gates serde_json with no cfg site and no flag to turn it on, and nothing notices when a feature stops having an effect.

Exit: every declared feature has a cfg site proved by a script, serde is optional behind a feature that actually serialises failures with secrets redacted, the json feature is either real or retired, and the matrix rows in CI are derived from the manifest rather than remembered.
