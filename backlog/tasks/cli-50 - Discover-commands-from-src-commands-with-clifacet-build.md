---
id: CLI-50
title: 'Discover commands from src/commands with clifacet::build'
status: Done
assignee: []
created_date: '2026-11-08 01:59'
updated_date: '2026-11-08 18:41'
labels: []
milestone: m-13
dependencies:
  - CLI-33
type: task
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 4, which reverses decision-15 item 2's refusal: discovery lives in clifacet behind a `build` feature, not in a third crate. The application's build script reports each folder's commands, groups and helpers; `clifacet::commands!()` with no list reads its own folder's report and emits `mod` declarations at the invocation site, so decision-15's objections — `#[path]` absolute paths and errors in OUT_DIR — do not arise, and no proc macro reads the directory (decision-15 item 4 stands). Measured in the review on the 2-core sandbox: clean dev +1%, clean release +7%, clean check +12% CPU (a fixed ~6 s), adding a command 0.8 s, a no-op rebuild 0.2 s.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `clifacet::build::commands()` behind a `build` feature, called from the application's build.rs, reports every folder's commands, groups and helpers through a versioned environment protocol and reruns when a folder changes; `commands_in(dir)` scans another folder
- [x] #2 `clifacet::commands!()` with no list finds its own folder with `Span::local_file()` resolved against the compiler's working directory, and the msrv job proves it at Rust 1.88
- [x] #3 `_name.rs` and `_name/` are private helper modules, visible to their folder and the groups under it and public only when `mod.rs` re-exports an item; a folder whose name starts with `_` is never a group
- [x] #4 Every mistake is reported at the application's own file: a syntax error in a helper, a command file without its struct, a helper reached from outside its folder (E0603), each covered by a test
- [x] #5 Dropping a file into src/commands adds a command with no other edit, and the build-time cost is re-measured and recorded
- [x] #6 Moving between `commands!(list, ..)` and `commands!()` changes only that call, and no new crate is published
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation, 2026-11-03. Decision-19 item 4 in two halves, neither of which the other can do alone.

**The build half — `crates/clifacet/src/build.rs`, behind `build` (off by default, a build script is its only caller).** `commands()` scans the calling crate's `src/commands`; `commands_in(dir)` scans any other folder. Each folder yields a `Report { folder, commands, groups, helpers, entries }`, and `Reports` collects one per folder, depth-first into every group. The wire is one line of `cargo:rustc-env`, because a value with a newline in it is silently truncated at the first newline:

```
clifacet-commands/1;folder=src/commands;command=2;group=1;helper=1;folder=src/commands/db;command=1
```

A `folder=` record opens a report; a `command`/`group`/`helper` before any `folder=` is a rejection, as is an unknown record and a version this crate does not speak (the message names both versions and suggests `cargo update`). Scan rules: `mod.rs` is the folder itself; a directory is a group, or a helper when its name starts with `_`; `<name>.rs` is a command, or a helper when its name starts with `_`; every name must be an identifier. `entries` — every scanned path — is *not* on the wire: it exists so the build script can emit `rerun-if-changed` for the folder **and each entry**. Emitting for `mod.rs` alone is not enough, and neither is emitting for each file without the folder: a *new* file changes the folder's mtime, not `mod.rs`'s.

**The macro half — `commands!()` with no list.** `Span::local_file()` gives the path the call was written in, relative to the compiler's working directory; the report's folder is crate-relative. The file *contains* the folder, so the resolution is a `split_once` on the folder text with the boundary required to be `/`, and the depth is counted from the remainder. The deepest matching report wins — `src/commands/db/migrate.rs` is inside both `src/commands` and `src/commands/db`, and taking the first match made a group's command register as `super::migrate` at the root. The module path is then `"super::".repeat(depth) + name`, which is what the report's names are relative to.

**AC#1.** 14 tests in `crates/clifacet/tests/build_report.rs`: the single-line encoding, the multi-folder tree, a round trip, a protocol mismatch, an unknown record, a command record before any folder, mixed helpers and groups, and the recursion of `commands_in`. Proven live in `examples/discovered` — a real crate with a real `build.rs` — where `list`, `echo` and the `db` group are found with no list written anywhere.

**AC#2.** The mechanism is proven on the developer toolchain: the fixture's `commands!()` names its own folder, groups included. The second half — Rust 1.88 — is a CI claim and this sandbox has no `rustup`, so it cannot be proven locally. `scripts/prove_msrv.py` reads its matrix from `scripts/check_features.py`, which derives the combinations from the manifest, so the new `build` feature is already one of the checked rows:

```
cargo +1.88 check --locked --all-targets -p clifacet --no-default-features --features build
```

That row is the proof, and it runs on this task's PR. `Span::local_file()` is what makes it work at all: proc-macro2 gates it on `rustc >= 88`, so 1.88 is not an arbitrary floor here, it is the first version where the mechanism exists.

**AC#3.** `_shared.rs` in the fixture is used by `list.rs` and visible to `src/commands/db` under it; `mod.rs` makes `greeting` public with `pub use _shared::greeting;` and nothing else is reachable. `_name/` is scanned as a helper directory by the same rule. A folder whose name starts with `_` is never a group — `a_helper_is_declared_but_never_a_command` asserts the count.

**AC#4.** Each mistake, and where it lands:

| Mistake | Reported at | Message |
| --- | --- | --- |
| unclosed delimiter in a helper | `src/commands/_shared.rs:11` | the compiler's own, in the helper file |
| a command file without its struct (`broken.rs`) | `src/commands/mod.rs`, the `commands!` call | `E0432 unresolved import broken` and `E0425 cannot find type Broken` — now a golden case, `tests/ui/commands_module_without_struct.rs` |
| a helper reached from outside its folder | the caller's line | `E0603 module _shared is private` — now a golden case, `examples/discovered/tests/ui/helper_from_outside.rs` |

The second one is the interesting case. It *could* point at the offending file, but it points at the application's own `mod.rs` — the line the application wrote — which is the promise AC#4 actually makes. `reexports` uses `quote_spanned!` on the module segment's span to get there; `pascal()` still builds the leaf ident at `call_site()`, so span gymnastics past this point would buy nothing. The missing-list path has its own trybuild case with a message that says what to write.

**AC#5.** Adding a file is the whole edit. `examples/discovered/src/commands/added.rs` with a `Added` struct taking an `Adding` input parsed to "added by existing" from `clifacet-discovered added` with no other change; the file was then removed. Timings on this sandbox, `examples/discovered`:

| Change | This measurement | The review's baseline |
| --- | --- | --- |
| no-op rebuild | 0.31 s | 0.2 s |
| removing a command | 0.43 s | — |
| adding it back | 0.42 s | 0.8 s |

`rerun-if-changed` fires both ways: after removing the command, `grep -c added` on the rebuilt binary is 0, and after restoring it, 1.

Three traps worth recording, because each cost a build:

- **A command file with no parameters still needs a distinct input type.** `input = Adding` beside `struct Added`; `input = Added` is `E0119 conflicting implementations of TryFrom<Added> for Added`.
- `input = self` and `input = Self` both fail with `E0435 non-constant value`.
- The file stem must equal the struct name in Pascal case: `added.rs` → `Added`.

**AC#6.** Moving from `commands!()` to `commands!(list, echo, db, …)` keeps all six discovery tests passing, and back again; the call is the only edit, and it was restored from `/tmp/mod_discovered.rs` afterwards. No new crate is published — discovery lives in `clifacet` behind the `build` feature (decision-19 item 4), so `clifacet-macros` and `clifacet` are still the only two.

**Docs.** `docs/derive.md` gained *Discovery: `commands!()` with no list* — the two halves, a table of which piece runs where, the four folder rules, and the three diagnostics — and the macro table grew a row for `clifacet::build::commands()`. `docs/getting-started.md` lists the `build` feature in the feature table (and what it costs: nothing in the binary, it is only compiled into a build script) and links discovery from "What to read next". `docs/testing.md` names the fixture's tests as the answer to "does a folder really register itself?". A stale sentence that said groups wait "until discovery can find them" was replaced, since discovery has now found them.

**Where this leaves the folder.** A hand-kept list stays fully supported — it is what a folder uses when its members are not one per file — and a discovered folder stays fully checkable by the compiler: the generated `Request` grows a variant per command, so a dispatch arm left out is still a compile error.

**AC#4, as tests.** Each case was first checked by hand against the fixture; two of the three are now golden cases that CI runs, because a promise about a diagnostic is only kept if something compares the text:

- `crates/clifacet/tests/ui/commands_module_without_struct.rs` — a module that does not hold the struct its name promises. `E0432 unresolved import broken::Broken` and `E0425 cannot find type Broken`, both pointing at the `commands!` line. Under discovery the same two items are generated from the same names, so one case covers both paths.
- `examples/discovered/tests/ui/helper_from_outside.rs`, driven by `examples/discovered/tests/privacy.rs` — `E0603 module _shared is private` at the caller's own line, with the note pointing at `src/commands/mod.rs`'s `mod _shared;`. This has to live in the fixture rather than in `crates/clifacet`: the message is about that application's helper, and an empty column to point at would prove nothing.
- The syntax error in a helper is the compiler's own, in the helper file, so pinning its text would pin rustc's wording rather than this crate's promise; that one stays hand-checked and is recorded above.

`examples/discovered/tests/reaching.rs` is now the positive half — the folder publishes exactly what `mod.rs` re-exports — because the negative half moved into the golden case above.

Documentation that came with the task: `docs/derive.md` (discovery section, macro table row), `docs/getting-started.md` (`build` in the feature table, a link under "What to read next"), `docs/testing.md` (the fixture as the answer to "does a folder register itself?"), and `README.md` + `docs/contributing.md` (the new crate in the layout tables). `.alint.yml`'s `cargo-workspace-members` rule lists the members on purpose — a new member is a reviewed change, not an automatic one — so the fixture is added there too.

**CI, PR #70** (`https://github.com/Archont561/clifacet/pull/70`), run `https://github.com/Archont561/clifacet/actions/runs/42500547939`:

| Job | Result |
| --- | --- |
| Minimum supported Rust version | pass, 1m09s |
| Repository and Rust quality | pass (Alint, docs, changelog, formatting, Clippy, feature boundaries, deny, audit) |
| Supported platforms — ubuntu / macos / windows | pass, 3m40s / 4m56s / 6m42s |
| Tests and source coverage | pass, 4m08s |
| Conventional commit messages, Classify the change, lock guard | pass |

**AC#2 is now complete rather than mechanism-only.** The msrv job passed with the new feature in its matrix, so `cargo +1.88 check --locked --all-targets -p clifacet --no-default-features --features build` — the row that carries `Span::local_file()` — compiles at the declared floor on a real 1.88 toolchain. That was the one thing the sandbox could not prove.

The first run of the quality job failed on `changelog-check`: the feature commit added an Unreleased entry and the committed `CHANGELOG.md` had drifted. Fixed with `pixi run changelog` (which needs a full clone — the sandbox checkout is shallow, so `git fetch --unshallow --tags` first) and committed separately as `chore(release): regenerate the changelog`.

2026-11-08: closed as Done. All six criteria were checked when the implementation landed (PR #70, run [42505760132](https://github.com/Archont561/clifacet/actions/runs/42505760132) green on every job including msrv), and the card stayed `In Progress` only because nothing prompted the status change — the same bookkeeping fault as CLI-49, found in the same sweep. No code changed here; the criteria were proven before this note was written.
<!-- SECTION:NOTES:END -->
