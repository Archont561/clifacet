# The documentation policy

Documentation is part of the product. A feature that a person cannot find out how to use
is not finished, and a page that describes behavior the code no longer has is a defect
with a file name — not a chore for later.

This page is the policy itself: what it asks of a contributor, why, and what enforces it.
[The docs index](../README.md) says which pages exist and where a new one goes.

## What it asks

**Treat documentation as part of the product.** A change is not done when the code works.
It is done when the behavior it introduces can be found, understood and reproduced by
somebody who was not in the conversation.

**One canonical explanation per topic.** Every fact has one home. A guide owns the steps
of a task, a concept owns the reason it behaves that way, the reference owns the promise.
Every other page links to that home; none of them restates it. A second copy is not a
convenience — it is the copy that will be stale first, and the reader has no way to know
which of the two is the true one.

**Document observable behavior.** What a caller can see is what the page describes: the
API, the streams, the exit codes, the files a command touches, the words it prints.
Internal structure belongs on a page only when it explains something the caller
experiences. A page is not a tour of the source.

**Write for the user, not the implementer.** The question a page answers is the reader's,
in the reader's order — how to do the thing, then why it behaves that way, then what is
guaranteed. The reasoning that led to the design belongs in a decision record; the page
says what is true now.

**Make it discoverable and navigable.** Every page is reachable from the index, every
section has a purpose, and no page is reachable only by guessing its address. A page nobody can
find is a page nobody will maintain.

**An outdated page is a defect.** Not a known issue, not a follow-up. If a change makes a
page wrong, the change fixes the page or records, in review, why the page is still
accurate. "There was no time" is not a reason a defect is allowed to ship.

## What a change has to do

1. **Assess the impact.** Before the pull request: does this change behavior, a flag, a
   default, an error, a guarantee, a supported platform, a command? If any answer is yes,
   at least one page is affected.
2. **Update the canonical page.** Not a new page beside it — the page that owns the
   topic, edited. If the topic has no canonical page yet, that is the moment it gets one.
3. **Link the new page** from the index and from wherever a reader arrives from.
4. **Remove what is no longer true.** An obsolete instruction left in place is worse than
   a missing one, because it is followed.
5. **Verify what the page claims.** Commands were run, examples compile, links resolve,
   names exist. The checks below do the mechanical part; the author does the rest.
6. **Say so in review.** If no documentation change was needed, the pull request says
   that explicitly and why. Silence is not an assessment.

## The definition of done

A documentation change is complete when all of the following hold. A pull request that
cannot tick them is not ready.

- [ ] The affected pages are updated, and no page contradicts the code.
- [ ] A fact that moved has one home; the old copy is gone rather than pointing at the new one.
- [ ] The new or changed page is linked from the index and from a page a reader would be on.
- [ ] Obsolete instructions are removed, not annotated.
- [ ] Every command in the prose was run, with the output the page shows.
- [ ] Every `` ```rust `` fence compiles, or is marked `rust,ignore` because it is an illustration.
- [ ] Every documented `clifacet::…` path names a public item that exists.
- [ ] Links and anchors resolve, in both directions.
- [ ] The prose passes Vale, with no new vocabulary entry unless the word is genuinely new.
- [ ] `pixi run --frozen --no-install automation docs` passes.
- [ ] If nothing needed changing, the review says so and why.

## What enforces it

The policy is not enforced by good intentions, so four checks run in CI and in the local
gate. Each answers a question a reader cannot answer by reading.

| Check | Answers |
| --- | --- |
| `alint` | does a page the repository promises to have still exist, and do the manifests still agree? |
| `markdownlint-cli2` | is the page structurally sound — heading levels, lists, fences, line length? |
| Vale | is it in this project's words: spelling, terminology, house style? |
| `scripts/check_docs.py` | does every link and anchor resolve, does every Rust fence compile, does every `clifacet::…` path name a real item? |

`scripts/check_docs.py` is the one that knows the most about this directory: it walks
`docs/` recursively, refuses a page outside the five sections, compiles every ```` ```rust ````
fence into a committed test file, and resolves every documented path against the crate's
public items. Vale and alint are configured from `vale.toml` and `.alint.yml`, both of
which carry the reason for each rule beside it.

What the checks cannot see is whether the page is *true*. A link can resolve to a page
that answers the wrong question, and a snippet can compile while the surrounding sentence
is wrong. That part is the definition of done above, and it is the author's.

## Who owns it

Everyone who changes the repository. There is no documentation role and no separate
review queue: the person who changes behavior is the person who knows what is now true,
and they are the cheapest person to write it down. A reviewer is expected to ask for the
page as they would ask for a test, and "no documentation change was needed" is a sentence
a reviewer can check.

History is the one thing that is not rewritten. [`backlog/`](../../backlog/) records
decisions as they were made and tasks as they were closed; a page here describes what is
true now. When a decision is superseded, the page changes and the decision stays.
