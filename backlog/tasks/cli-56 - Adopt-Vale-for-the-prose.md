---
id: CLI-56
title: Adopt Vale for the prose
status: Done
assignee: []
created_date: '2026-11-08 17:19'
updated_date: '2026-11-08 18:01'
labels: []
milestone: m-13
dependencies: []
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-20: adopt Vale for the prose and keep alint for the repository. vale.toml is authored; scripts/prose.py renders .vale.ini and .vale/styles/; prose-check runs in test-local, quality and the pre-commit hook.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Vale lints README.md, AGENTS.md and docs/**/*.md, with the Microsoft and write-good styles pinned to release URLs
- [x] #2 vale.toml is the authored source and `prose-check` fails when .vale.ini or .vale/styles/ has drifted from it
- [x] #3 Every inherited rule the repository sets its own way is declared in vale.toml with the reason, and a `NO` without one is refused
- [x] #4 The repository's own style carries the American-English substitutions the prose needs, and the vocabulary the names
- [x] #5 prose-check runs in test-local, quality and the pre-commit hook, and works offline
- [x] #6 AGENTS.md says which of Vale and alint owns a finding, and docs/development.md explains how to change the check
- [x] #7 The prose the check found inconsistent is corrected without touching a code span or an API name
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**The lint ran, and the configuration was wrong.** The first CI run failed on the prose step, and the cause was not the prose: `vale sync` cannot install a package here (GitHub's release assets are blocked), so `Microsoft` and `write-good` were named in `BasedOnStyles` and absent from the StylesPath. Vale stopped with `E201 Invalid value` before reading a page.

Fixed by **committing the two styles** under `.vale/styles/`, with their MIT licences, at the versions the URLs pin — the same shape `vale sync` produces. A gate that needs a download fails on the offline restore this repository runs most of its checks in, and this one is in `test-local`. The packages are still pinned to release URLs so a review can see what is installed and from where.

A Vale 3.26.0 binary was then obtained from `@taskless/vale-linux-x64` on npm — the only reachable path, since conda channels and GitHub release assets are both blocked — and the check was run locally until it passed. The first real run: **49 errors, 301 warnings**. Each answered on its merits:

| Finding | Count | Answer |
| --- | ---: | --- |
| `write-good.Passive` | 250 | off: technical writing says what a system does to a value, and the actor is the library |
| `Microsoft.Adverbs` | 32 | off: "deliberately" and "explicitly" mark the places a design refuses something on purpose |
| `Vale.Spelling` | 40 | vocabulary: `impl`, `parser`, `relock`, `scrollback`, `trybuild`, `tui` and the rest are real words |
| `Microsoft.Quotes` | 6 | **the prose was wrong** — `.` and `,` sat outside the closing quote; fixed |
| `Microsoft.Terms` | 6 | off: the reference style wants "personal digital assistant" for "agent", which is not the sense here |
| `write-good.Weasel` | 5 | off: "usually" is how a rule with exceptions is stated honestly |
| `Microsoft.FirstPerson` | 4 | fixed: a table of navigation questions was written in the first person |
| `Microsoft.HeadingPunctuation` | 2 | off: a heading here is a question, and the rule targets a heading that became a sentence |
| `write-good.So` | 2 | fixed: a sentence opening with "So", and a `; so` construction |
| `Microsoft.GeneralURL`, `Microsoft.Auto` | 2 | fixed: "address" for a run's location, "auto-installed" unhyphenated |

Result: **0 errors, 0 warnings, 0 suggestions over 11 pages**, both locally and in CI.

**`lint` had a bug the local run exposed.** It parsed Vale's CLI table by splitting on `:`, which mistook the table's right-hand column for the rule name — so a real failure reported "Vale did not run" and printed no rule counts. It now reads `--output=JSON`, which names the check, the line and the message, and the tests use a fake `vale` that prints that shape. Verified by hand: a planted `behaviour` and a nonsense word produced two annotations on the right lines and a summary counting one alert per rule.

**CI, run 42507086937:** Repository and Rust quality (which runs the prose step), Minimum supported Rust version, Tests and source coverage, and all three platform jobs pass. The relock bot had already committed the `vale 3.26.0` lock entry in `0b147dff`.

**Session-start rule.** `AGENTS.md` now opens with the check the maintainer asked for: confirm `pixi` exists, and when it does not, run `scripts/bootstrap.sh` — which restores the sandbox, then installs the lefthook hooks — and confirm both before working. The order is what `bootstrap.sh` actually does (`sync_checkout`, `restore_environment`, `install_hooks`, `smoke_checks`), not an assumption. `.agents/skills/session/SKILL.md` carries the same step, so a session that begins without history still finds it.
<!-- SECTION:NOTES:END -->
