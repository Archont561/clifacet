---
id: decision-17
title: Plugins as `<cli>-<name>` binaries on PATH
date: '2026-11-01 23:05'
status: rejected
---
## Context

The request: a binary called `<cli_name>-<something>` should be exposed automatically as
`<cli_name> <something>`, the way `git-lfs` becomes `git lfs`, `cargo-nextest` becomes
`cargo nextest`, and `kubectl-foo` becomes `kubectl foo`.

Two accepted decisions bear on it:

- **decision-1** forbids hidden global registration. Discovery must therefore be something
  the application asks for, in its own source, not something that happens to it.
- **decision-15** rejected run-time dispatch for *commands*, because `clap` needs the whole
  tree to parse and an `external_subcommand` hands back untyped `OsString`s, which would
  break "every value the command receives is validated".

A plugin is a different thing from a command, and the distinction is the whole reason this
is compatible: **a plugin's arguments belong to the plugin.** The host validates its own
input; the plugin's argv is never turned into a typed input and never claimed to be
validated. The invariant is scoped, not weakened.

## Measured

Compiled against `clap` 4.6.7 in this repository, then reverted. Seven properties held:

1. **A discovered binary becomes a subcommand whose arguments pass through verbatim.**
   `cast deploy --force -x value` yields exactly `["--force", "-x", "value"]` with
   `Arg::new("arguments").num_args(0..).trailing_var_arg(true).allow_hyphen_values(true)`.
2. **`--help` reaches the plugin, not `clap`.** With `disable_help_flag(true)`, `cast deploy
   --help` hands `["--help"]` to the plugin, so the plugin documents itself.
3. **The `--` separator is consumed by `clap`**, not forwarded: `cast deploy -- --force`
   arrives as `["--force"]`. A plugin that needs a literal `--` gets one only if the host
   re-inserts it — a rule the implementation has to state.
4. **Discovery executes nothing.** A plugin script that creates a marker file on run was
   discovered, listed, and never executed.
5. **A built-in name is never shadowed** when the discovered list is filtered against the
   built-in subcommand names.
6. **Plugins appear in `--help` with their path** when each runtime subcommand gets an
   `about` of `(plugin) /usr/local/bin/cast-deploy` — the reader can see what will run.
7. **A derive-built tree and runtime subcommands coexist.** Parsing `cast greet --name ada`
   still converts through `Cli::from_arg_matches`; parsing `cast deploy --force` yields the
   plugin name and its arguments, and the derive correctly *fails* to convert it — which is
   the interception point the host uses.

One cost found by the compiler: `Command::new(name)` takes `clap`'s `Str`, so a run-time
name needs **`clap/string`**. Without it the prototype had to `Box::leak` the name. The
feature must be enabled for plugin support rather than leaked around.

## Decision

Ship `clifacet::plugins`, behind a `plugins` Cargo feature, with four parts:

```rust,ignore
// 1. discovery — explicit, over directories the application names.
let plugins = clifacet::plugins::discover("cast", std::env::var_os("PATH"));

// 2. the tree — built at run time, then parsed.
let mut command = Cli::command();
clifacet::plugins::append(&mut command, &plugins);
let matches = command.get_matches_from(argv);

// 3. routing — a plugin name is intercepted before the derive sees it.
if let Some((name, arguments)) = clifacet::plugins::selected(&matches, &plugins) {
    return clifacet::plugins::run(plugin, arguments);   // exit status passthrough
}
let cli = Cli::from_arg_matches(&matches)?;
```

- **Discovery is explicit.** `discover(prefix, paths)` takes the directories; assembling
  `PATH` is the application's call, so a library user can offer `--plugin-dir` instead.
  Nothing is scanned implicitly, and nothing is probed with `--help` (that would execute).
- **Names are single-level.** `<prefix>-db-list` becomes the subcommand `db-list`, not
  `db list`: a legitimate hyphen in a name cannot be told from a separator without a
  convention the plugin would have to declare anyway. A plugin that wants nesting nests
  inside itself.
- **Built-ins always win.** A discovered name that collides with a compiled subcommand is
  dropped, and the drop is reported (`plugins::discover` returns them separately), because
  silently ignoring a plugin is how a user spends an afternoon debugging a stale build.
- **A plugin is terminal.** Its arguments are not parsed, not validated and not recorded by
  clifacet; the host forwards them and passes the exit status through. `plugins::run`
  spawns with inherited stdio, and on Unix may `exec` instead — with the documented
  consequence that the host's own exit mapping no longer applies.
- **Plugins are outside the mode contract.** A separate process decides its own
  interactivity, so `Policy` cannot make it non-interactive. The host may refuse to launch
  plugins when its plan is headless (`plugins::allowed(plan)`), and that is the only
  influence it has; the limitation is documented rather than papered over.

## Consequences

- **`clap/string` becomes a dependency of the feature.** Enable it with `plugins`, not
  unconditionally, so an application that never discovers plugins keeps the smaller build.
- **Windows differs and must be tested there.** Discovery matches `PATHEXT` (`.exe`,
  `.cmd`, `.bat`), names compare case-insensitively, and `.cmd`/`.bat` need a shell to run.
  The prototype is Unix-only; the feature is not shippable until the Windows job exercises
  it, which the `Supported platforms (windows-latest)` job can.
- **The `--` rule and the pass-through rules need a documented table**, because they are
  the kind of detail that produces silent argument corruption: verbatim forwarding,
  `--` consumed once, `--help` belonging to the plugin, exit status passed through.
- **Tests**: a scratch directory of fake executables for discovery, help and shadowing; a
  `#[cfg(unix)]` process test for exit status and argument delivery; a fake-runner seam so
  the routing logic is testable without forking.
- **It pairs with decision-15 but does not depend on it.** Plugins can be added to the
  current single-command CLI with a `Command` built by hand; the declarative tree simply
  gives them something to hang off.

## Rejection note (2026-11-06)

**Rejected as a library API** by the maintainer on 2026-11-06; **a documented recipe is
accepted instead.** The measurements above stand and are what the recipe is built from.

- **Why not an API.** By this decision's own account a plugin's arguments are not
  validated, it is outside the mode contract and it takes no part in parity — everything
  clifacet exists to provide. What would remain in the crate is a process launcher with
  Windows `PATHEXT` and `.cmd` handling and the `clap/string` feature. That is the same
  shape as decision-18's refusals (watch mode, completion generation, config layering):
  work that `clap` and a few lines of application code already do.
- **What is kept.** A section of the authoring guide with a compiled snippet: discovery
  over directories the application names, one passthrough subcommand per plugin with its
  path in the help, routing before the derive converts, the forwarding table (`--` consumed
  once, `--help` belonging to the plugin, verbatim arguments, exit status passed through),
  and refusing to launch a plugin when the plan is headless. CLI-29 carries it; clifacet
  gains no `plugins` module, feature or dependency.
