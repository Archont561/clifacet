# Clifacet documentation

Clifacet is a Rust library for command-line presentation modes: one command core,
rendered through flags, prompts, a full-screen form, or a screen the application draws
itself. The core, the prompts, the companion derive and the TUI are implemented; the JSON
frontend is reserved and not implemented.

This directory is the product's knowledge layer, not a byproduct of it. What follows is
how it is organized and what keeps it true; a change that contradicts either is a defect
in the change.

## Start here

| To | Read |
| --- | --- |
| add the crate and write a first command | [Getting started](guides/getting-started.md) |
| copy a working command | [The simple example](../examples/simple/README.md) |
| understand why a run asked or did not ask | [Modes and the interaction policy](concepts/modes.md) |
| give an existing `clap` command validated input | [The companion derive](guides/derive.md) |
| draw your own field or screen | [TUI composition](guides/tui-composition.md) |
| prove a promise without a terminal | [Testing the command core](guides/testing.md) |
| look up a seam, a stream or an exit code | [Command core and its seams](reference/command-core.md) |
| work on this repository | [Contributing](development/contributing.md), then [Development](development/development.md) |
| understand the hooks, and why CI takes the time it does | [The feedback loop](development/feedback-loop.md) |

## How this directory is organized

Five directories, and a page's directory says what the page is for.

| Directory | Holds | The question it answers |
| --- | --- | --- |
| [`guides/`](guides/) | tasks, in the order a reader meets them | how is this done? |
| [`concepts/`](concepts/) | the model a reader has to hold to predict behavior, including [the documentation policy](concepts/documentation-policy.md) | why does it do that? |
| [`reference/`](reference/) | the seams, the guarantees, the tables | what exactly is promised? |
| [`architecture/`](architecture/) | [how the repository itself is built](architecture/repository.md), as opposed to the library | how does it work inside? |
| [`development/`](development/) | working on this repository: checks, hooks, release | how is it changed? |

Every directory holds its pages directly; `docs/README.md` is the only page at the top
level. `scripts/check_docs.py` enforces the list: a page in a directory that is not one of
the five fails the documentation gate, so a new section is a decision rather than an
accident.

## One explanation, kept in one place

The rule that keeps this directory usable is that a fact has one canonical home. The
home for a task is a guide, for a reason is a concept, for a promise is the reference.
Elsewhere it is a link.

That is why the README of this repository introduces and links instead of explaining, why
`command-core.md` owns the tables of streams and exit codes that the guides cite, and why
the same paragraph does not appear on two pages. When a page needs something another page
already says, it links to it; when the wording is wrong, the page that owns it is edited
and every other page follows by linking. A second copy is not a convenience, it is the
copy that goes stale first.

The policy behind all of this — what a change owes the documentation, the definition of
done, and what enforces it — is [The documentation policy](concepts/documentation-policy.md).
It is the canonical explanation; this page does not restate it.

Planning is deliberately elsewhere: [`backlog/`](../backlog/) holds decisions, milestones
and tasks. Those are the project's direction, not a description of the product, and they
are not rewritten when the product changes.

## What keeps it true

Nothing here is trusted because someone wrote it carefully. Four checks run in CI, and
each answers a question a reader cannot:

| Check | Refuses to let |
| --- | --- |
| `alint` | a page the repository promises to have go missing |
| `markdownlint-cli2` | a page whose structure is broken — skipped heading levels, malformed lists, a fenced block nobody closed |
| [Vale](https://vale.sh) | a page in the wrong words: misspellings, invented terminology, a house style that drifts page by page |
| `scripts/check_docs.py` | a link that goes nowhere, an anchor no heading provides, a Rust fence that no longer compiles, a `clifacet::…` path that names nothing |

`scripts/check_docs.py` is the oldest of the four and the one that knows the most about
this directory: every `` ```rust `` fence is written into a committed test file and
compiled, and every documented path is resolved against the crate's public items. Links
in a page are checked against the file they name and the headings it has.

## Adding or changing a page

1. Decide which of the five directories it belongs in, and add it there.
2. Link it from this index, and from whatever page a reader would arrive from.
3. If it moves a fact, remove the old copy rather than leaving both.
4. Run `pixi run --frozen --no-install automation docs`. It renders the snippet test file
   when a page's Rust changed (`automation docs_write` regenerates it), and it names every
   link that no longer resolves.
5. Run `pixi run --frozen --no-install automation prose` for the words.

A page is finished when its commands were run, its examples compile, and its links
resolve. See [Contributing § Documentation is checked](development/contributing.md#documentation-is-checked)
for the definition of done a pull request is held to.
