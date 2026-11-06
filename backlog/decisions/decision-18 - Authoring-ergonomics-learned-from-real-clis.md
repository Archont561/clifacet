---
id: decision-18
title: Authoring ergonomics, learned from Backlog.md, citty and typer
date: '2026-11-02 00:40'
status: accepted
---
## Context

The request was two things at once: *"how could I make the backlog.md CLI with clifacet with
all options and even better handling"*, and *"look at other CLI tools ... to find what useful
features I should implement to make CLI authoring easy and enjoyable and developer friendly,
with the most common flows handled"*.

The first half is answered by a working artefact, not by prose: `examples/backlog-lite` is a
`bl` wrapper over the Backlog.md CLI, written against the published API only, with thirteen
behaviour tests and a live run against the real tool. Everything this decision proposes came
out of that attempt — features whose absence cost the wrapper something concrete — plus a
read of the CLI it wraps and two mature authoring libraries.

The rule for inclusion is the strict one: **a feature earns a place only if an application
had to write it by hand here.** A list of everything `typer` does would be a wish list, not a
design.

## Evidence

**Backlog.md 1.53.0** (surveyed through its own `--help`): every query command carries
`--plain`, `--json` (a versioned schema, `schemaVersion: 1`), `--watch` (JSON only),
`--max-count`, `--skip`, `--count`. `task list` has sixteen filters, including repeatable-or-
comma-separated `--status`, mutually-exclusive `--assignee`/`--unassigned`, closest-match
case-insensitive `--milestone`, and a computed `--ready`. `task edit` distinguishes replacing
a value from `--add-label`/`--remove-label`/`--clear-labels`, and edits acceptance criteria by
**1-based index** (`--check-ac`, `--remove-ac`). Its help prints `Input schema` blocks —
required, optional, reads, output, examples — because the tool is also driven by agents.

**citty 0.2.2**: arguments are data (`{ type: positional|string|boolean|enum, required,
default, alias }`), subcommands are a map with lazy imports, and `--no-` negation is generated.
**typer 0.27.3**: `typer.Option(help, prompt, confirmation_prompt, min, max, clamp, envvar,
hidden, show_default, metavar)`, with the docstring as help, and `Annotated` carrying it.

**What the wrapper needed and did not have**, measured while writing it:

1. **A question that is not free text.** `Prompter` offers `ask` and `confirm` and nothing
   else, so a status, a milestone, or a choice between two existing values is collected as
   a string and re-validated by the application. The wrapper did exactly that, by hand.
2. **A confirmation policy that separates "no" from "cannot ask".** `Session::confirm`
   returns `Ok(false)` when nothing may ask, which is the right fail-safe but indistinguishable
   from a human declining. The wrapper had to inspect `session.plan().asks_the_user()` itself
   to report "pass `--yes`" instead of "declined".
3. **Provenance for free.** `Provenance::record` is called by the application; nothing derives
   it from the parse result, so a wrapper with a custom tree records nothing.
4. **A plan-shaped seam.** The wrapper's best property is that every leaf produces an
   argument vector and *nothing* touches the process until one `run`. `--dry-run` then costs
   four lines and cannot drift from the real path. clifacet has `Command::execute` and no
   equivalent "here is what I would do" value, so this was hand-rolled.
5. **Repeatables and enumerated values from clap.** Forwarding `--ac` repeatedly, and
   validating `--status` against the words the wrapped tool uses, is ordinary `clap` work —
   the wrapper did it and it was fine. The *documentation* of it is what hurts.

## Decision

Adopt seven authoring features, in this order. Each one is a public API addition; none changes
the command-core contract.

1. **Choice prompts.** Extend `Prompter` with
   `choose(&mut self, question: &str, choices: &[Choice]) -> Result<Choice, Failure>`, where
   `Choice` carries a value, a label and optional help. A `Session::choose(supplied, ...)`
   wrapper returns the supplied value untouched, so parity is preserved. This is the single
   biggest gap the wrapper hit: a status, a milestone, an assignee and a "which task?" are
   all multi-choice questions, and the `inquire`-backed frontend already supports them.
2. **Secret and multi-value questions.** `Session::ask_secret` (currently documented as
   unsupported) and `Session::collect_values`, which repeats a question until the user
   stops — the prompt-side twin of a repeatable flag. Both route through the same
   `Provenance` recording as `ask`, so a value's origin stays answerable.
3. **A tiny confirmation policy enum.** `Confirm::Asked`, `Confirm::Assumed` (from `--yes`),
   and `Confirm::Unavailable`. `Session::confirm` keeps its fail-safe behaviour but returns
   the classification, so "the user said no" and "nothing could be asked" stop being the same
   value. Applications that only want a boolean call `.is_confirmed()`.
4. **`Provenance::from_matches`.** A function that walks a `clap::ArgMatches` and records
   `Source::Argv`/`Source::Default`/`Source::Environment` for every present value, so
   provenance is a property of the framework rather than of diligence. Prompted values are
   added by the session, so the two sources compose.
5. **A plan value for mutations.** `Command::plan(input) -> Plan` with `Plan::describe()` and
   `Plan::execute()`, so `--dry-run` is one call in every application instead of a bespoke
   argument-vector printer. The wrapper's `plan_of`/`shell_line` pair is the reference
   rendering, including the minimal-quoting rule.
6. **Parity sugar.** A `parity!`/`#[rstest]`-friendly form of `Parity` construction, because
   the wrapper's thirteen tests each closed with three lines of `Outcome` plumbing that no
   reader learns anything from. Behaviour, not assertion text, is what these tests are for.
7. **`report::render`.** One place that writes a `Failure` with its kind, its message and, for
   a usage failure, the usage text — the mapping application `main`s keep rewriting.

The declarative syntax (decisions 13–16) and the field attribute (CLI-28) are orthogonal and
keep their own slots: they decide how input is *declared*, this decision decides what an
application can *ask* and *show*.

## Refusals

- **No `--watch` equivalent.** Live-reloading output is a presentation concern and would put a
  loop inside the command core.
- **No completion, man page, or versioned JSON schema generator in this tranche.** All three
  are worth having and all three are downstream of a stable `Plan`/contract; `clap_complete`
  is already reachable by applications.
- **No configuration-file layering yet.** `Source::Config` exists in the enum and nothing
  fills it; inventing a config format before a second application needs one is how a
  framework acquires a format it cannot change.
- **No comma-separated multi-value flags generated automatically.** Backlog.md accepts
  `--status a,b`; `clap`'s `value_delimiter` already does that, and silently splitting every
  repeatable argument would break values that legitimately contain a comma.
- **No index-addressed editing helpers.** `--check-ac 2` is a property of *that* tool's data
  model, not of CLI authoring.

## Consequences

- Items 1–3 are additive to `prompt` and are the ones the reference example should adopt, so
  the `tui` form gains a choice field rather than a text field with a parser that rejects.
- Item 4 needs a `clap` dependency boundary already present, and must be tested against
  defaults, environment variables and absence — the three cases where an eager implementation
  records a lie.
- Item 5 introduces a second execution path, which is exactly the drift risk the parity
  harness exists to catch: `Plan::execute` should be the *only* path, so no drift is possible.
- Every item gets a trybuild or behaviour test, and `examples/backlog-lite` is the acceptance
  case: it should lose code when the features land, not merely gain calls.
- The wrapper itself stays an example crate (`publish = false`, a workspace member), so it is
  compiled and tested by CI but never published; it is the second worked example the project
  has and the first that wraps somebody else's CLI.

## Acceptance note (2026-11-06)

Accepted by the maintainer on 2026-11-06 "as suggested". The Decision above is left as it
was proposed; this note records the verdict per item.

- **Items 1, 2, 3, 4 and 7 are accepted.** Items 1–3 (`choose`, `ask_secret` with
  `collect_values`, the classified confirmation) are CLI-30; items 4 and 7
  (`Provenance::from_matches`, `report::render`) are CLI-31.
- **Item 5 (`Command::plan` for `--dry-run`) is accepted but built last**, after the rest of
  CLI-31, because it is the only item that changes how a run is executed.
- **Item 6 (parity-test shorthand) is deferred.** The parity tests it would shorten are few
  and readable as they are; it can be proposed again if a third worked example repeats the
  pattern.

## Implementation note for items 1–3 (2026-11-06, CLI-30)

Where the shipped API differs from the Decision above, and why:

- **`Prompter::choose` returns a position, and `Choice` carries no value.** A value-carrying
  `Choice<T>` would make the method generic, and a generic method cannot be called through
  the `Box<dyn Prompter>` a `Session` holds. So a frontend sees `Choice` labels (with
  optional help) and returns an index; `Session::choose<T: Display>(supplied, question,
  options)` keeps the values, labels them by `Display`, and maps the index back, failing
  on one it never offered. Passing help through `Session::choose` is not offered yet.
- **New `Prompter` methods are provided, so existing frontends keep compiling.** `choose`
  defaults to a numbered list read through `ask`. `ask_secret` defaults to a *refusal*:
  falling back to `ask` would echo the credential. `Terminal` keeps the refusal, because
  a line reader cannot switch echo off; `Dialog` asks with a hidden `inquire::Password`.
- **The enum is `Confirmation`, with four variants.** "Asked" has to carry the answer, so
  it is `Confirmed`/`Declined` beside `Assumed` and `Unavailable`; `Confirm` would also
  collide with `inquire::Confirm` where the dialog uses both. Changing the return type of
  `Session::confirm` is breaking, which the proposal implied despite its "public API
  addition" framing; `is_confirmed()` is the migration.
- **The session keeps its own `Provenance`.** Answered `ask_for`, `ask_secret` (flagged
  secret) and `collect_values` record `Source::Prompt`; a supplied value is not recorded,
  because where it came from is item 4's job (`Provenance::from_matches`, CLI-31).
- **`collect_values` collects strings.** An empty or blank answer ends the list, and the
  question is not reworded, so it should say how to finish. A failure part-way discards
  what was gathered.
- **backlog-lite's status filter asks only for a bare `--status`.** `bl list` is the
  default verb and must not grow a question; `--status` with no value asks, and is
  refused as incomplete where nothing can ask.
- **Not in CLI-30:** the `tui` form's choice field (a Consequence above), `ask` on a
  `secret` field in the derive, and typed `collect_values`.

