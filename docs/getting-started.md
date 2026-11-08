# Getting started

Clifacet keeps one command core and renders it through four frontends: flags, a
prompt, a full-screen form, and whatever the application draws itself. The command
never writes a stream and never reads a terminal — that is what makes the same
command reachable from all of them, and testable without any of them.

## Add the dependency

```toml
[dependencies]
clifacet = { version = "0.3.0", features = ["tui"] }
clap = "4.6.7"
```

### Features

Every feature is off by default, and each one is a dependency boundary: a feature you
do not enable is not compiled, and neither are the crates it would bring in.

| Feature | What it adds | What it costs |
| --- | --- | --- |
| *(none)* | commands, the derive, the policy, the plain frontend, secrets, provenance and the harness | `clap`, `thiserror` and the `clifacet-macros` proc macro |
| `interactive` | the `inquire`-backed dialog behind `Prompter`, so prompts work at a terminal | `inquire` with its `crossterm` backend — the largest prompt-side tree |
| `tui` | the full-screen form in `clifacet::tui` and the TUI mode in the policy | `ratatui` and `crossterm` — the largest tree of the three |
| `serde` | `Serialize` for `Failure` and `FailureKind` (`{"kind":"validation","message":"…"}`) and for `Secret`, which serialises as `[redacted]` | `serde` and its derive macro, a proc macro built at compile time |
| `build` | `clifacet::build::commands()`, the build-script half of discovery: a folder registers its own commands, so adding a command is adding a file | nothing in the binary — the module is only compiled into a build script |

`serde` is how an application reports failures as JSON: it serialises a `Failure` with
`serde_json` under a flag of its own, on a stream of its own, because output is the
application's (decision-3) and the library writes no JSON itself. There is no `json`
feature — 0.2.0 reserved one that selected `serde_json` and did nothing else; it was
retired in favour of `serde`, so an application that wants JSON depends on `serde_json`
directly and enables `serde` here.

Every combination is checked in CI — see
[Supported platforms and feature combinations](development.md#supported-platforms-and-feature-combinations).

## Write the command

A `Command` is a value that takes validated input and returns data or a failure. It
is the only seam the frontends share, so it stays deliberately small:

```rust
use clifacet::{Command, Failure};

struct Greet;

impl Command for Greet {
    type Input = String;
    type Output = String;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        Ok(format!("hello, {input}"))
    }
}

let greeting = Greet.execute("ada".to_owned()).expect("a greeting");
assert_eq!(greeting, "hello, ada");
```

`Input` is built *after* validation, so `execute` has nothing left to reject, and
`Output` is data rather than a message: whether it becomes a line of text, JSON or a
table is the application's decision.

## Run it

Everything around the command — parsing, asking for what the flags left out,
validation, which stream the data goes to, and the exit code — is one pipeline, and
the runner owns it. Give the command line its input type with
[the companion derive](derive.md), make it the command, and `main` is one line:

```rust
use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Command, Failure, Globals};

#[clifacet::command(input = Greeting)]
#[derive(Clone)]
#[command(name = "greet", about = "Say hello")]
struct Greet {
    /// Who to greet.
    #[clifacet(long, ask = "Who should I greet?", fill = "world")]
    name: Option<String>,

    /// `--yes` and `--dry-run`, which every application gets.
    #[command(flatten)]
    #[clifacet(skip)]
    globals: Globals,
}

struct Greeting {
    name: String,
}

impl Command for Greet {
    type Input = Greeting;
    type Output = String;

    fn execute(&self, input: Greeting) -> Result<String, Failure> {
        Ok(format!("hello, {}", input.name))
    }
}

// The binary's whole `main`:
fn main() -> std::process::ExitCode {
    clifacet::run::<Greet>()
}

// `run_with` is the same pipeline with the process taken out — the arguments, what the
// terminal can do, the frontend that answers questions and both streams are parameters
// — which is how a test runs the application without a terminal.
let (mut out, mut err) = (Vec::new(), Vec::new());
let code = App::<Greet>::new().run_with(
    ["greet", "--name", "ada"],
    Capabilities::headless(),
    || -> Box<dyn Prompter> { Box::new(Scripted::default()) },
    &mut out,
    &mut err,
);
assert_eq!((code, out.as_slice()), (0, b"hello, ada\n".as_slice()));
```

The runner takes the steps in this order, and whatever stops a run stops it before
the command changes anything:

| Step | Stops with | Exit code |
| --- | --- | --- |
| parse | the help or version that was asked for, on stdout | `0` |
| parse | a usage error, on stderr | `2` |
| settle the mode | a mode the policy forbids or the terminal cannot host | `2` |
| ask | a declined question | `130` |
| validate | a value the bridge refuses | `2` |
| run, or preview with `--dry-run` | the command's failure | `1` (`2` for validation) |
| write the output | a data stream that cannot be written | `1` |

The numbers are `ExitPolicy::standard()`'s; see
[Command core § Streams and exit codes](command-core.md#streams-and-exit-codes) for what each one means.

### What an application changes

`clifacet::run::<Cli>()` is `clifacet::App::<Cli>::new().run()` with every default.
Build an `App` to change one:

| Method | What it changes | Default |
| --- | --- | --- |
| `.policy(..)` | how interactive a run may be, and is unasked ([modes](modes.md)) | prompts when a terminal can answer them |
| `.mode_from(..)` | reads the mode the user asked for — an `--interactive` or `--tui` flag of the application's own | none asked for |
| `.command(&other)` | runs another command with the same input — how a test records a run | the parsed command line |
| `.context(..)` | how to build what the command depends on ([below](#what-a-command-depends-on)) | none: every command runs as a `Handler` of `()` |
| `.render(..)` | how the output is written: a `Render` value, or a closure taking the output and the stream | `Plain`, the output's `Display` and a newline |
| `.exit(..)` | the exit code each outcome costs | `ExitPolicy::standard()` |

`examples/simple`'s `app()` is one of these, and its `main.rs` is `app().run()`.

### What a command depends on

Input is what the user supplied, and every value of it has to be something the user could
type. A store, a client, a clock or a config file is not: the command *depends* on it, and
the application already has it. Passing one through the input would mean a field that is
not a flag, a global or a `thread_local!`, and a command that cannot be run twice with
two different ones. That is the application's context, and it travels beside the input:

```rust
use clifacet::command::{Plan, PlannedWith};
use clifacet::prompt::Scripted;
use clifacet::{App, Failure, Globals};

/// What the user supplied: the task to mark done.
#[clifacet::command(input = DoneInput)]
#[derive(Clone)]
#[command(name = "done")]
struct Done {
    /// The task to mark done.
    #[arg(long)]
    task: String,

    #[command(flatten)]
    #[clifacet(skip)]
    globals: Globals,
}

struct DoneInput {
    task: String,
}

/// What the application depends on: the store of tasks, opened once, outside the input.
struct Store {
    open: Vec<String>,
}

/// The command plans with the store; `#[clifacet::planned]` makes it a `Handler<Store>`.
#[clifacet::planned]
impl PlannedWith<Store> for Done {
    type Input = DoneInput;
    type Output = String;

    fn plan<'a>(&'a self, input: DoneInput, store: &'a Store) -> Result<Plan<'a, String>, Failure> {
        if !store.open.contains(&input.task) {
            return Err(Failure::validation(format!("there is no open task named {}", input.task)));
        }
        let step = format!("mark {} done", input.task);
        let task = input.task;
        Ok(Plan::new(move || Ok(format!("{task} is done"))).step(step))
    }
}

let store = Store { open: vec![String::from("CLI-47")] };
let app = App::<Done>::new().context(move || Ok::<_, std::io::Error>(Store {
    open: store.open.clone(),
}));
let (mut out, mut err) = (Vec::new(), Vec::new());
let code = app.run_with(
    ["done", "--task", "CLI-47", "--dry-run"],
    clifacet::policy::Capabilities::headless(),
    || Box::new(Scripted::new([])),
    &mut out,
    &mut err,
);
assert_eq!((code, out.as_slice()), (0, b"mark CLI-47 done\n".as_slice()), "{err:?}");
```

`App::context` takes a function returning the context, not the context itself, because the
runner decides *when* to call it. The three rules that follow from that:

- **After the input is valid, on first use.** A question may need the context — the
  example above could offer `store.open` as the choices for `--task` — so it is built by
  whichever asks first, and once per run: the command runs with the same value the
  question read. Nothing that stops a run earlier pays for it: not `--help`, not
  `--version`, not a usage error, not a rejected value, not a declined question.
- **A failure to build it is an execution failure.** It is reported as
  `cannot prepare the application: {error}`, on the diagnostic stream, with exit code `1`,
  and nothing runs.
- **A question reads it with `session.context::<Store>()`.** That is why the type has to
  outlive the run (`Store: 'static`), while the handle is an `Rc` — so the session is free
  for the question itself. Ask only when the question will be asked:

  ```rust
  use clifacet::Failure;
  use clifacet::prompt::Session;

  struct Store {
      open: Vec<String>,
  }

  /// A `collect` function: it turns the parsed command line into the input, asking for
  /// what the flags left out.
  fn pick_task(chosen: Option<String>, session: &mut Session) -> Result<Option<String>, Failure> {
      // Nobody can answer — a piped run, or `--yes` — so the context is not built for a
      // question that will not be asked.
      if chosen.is_some() || !session.plan().asks_the_user() {
          return Ok(chosen);
      }
      // The same store the command runs with: built here, because this asks first.
      let store = session.context::<Store>()?;
      session.choose(None, "Which task?", store.open.clone())
  }
  ```

  A session with no context of that type is a declaration mistake, and says so rather
  than guessing: the message names the type it was asked for.

Every `Command` is a `Handler` of any context through the library's blanket impl, so an
application that needs none is written exactly as before, and a command in a tree may take
the context while its siblings do not: `#[clifacet::subcommands(.., context = Store)]`
threads `&Store` to the chosen leaf, and a leaf without one still runs.

### The options every application gets

`clifacet::Globals` is `--yes`, which answers every confirmation instead of asking, and
`--dry-run`, which shows what would run and runs nothing. Both are global, so they work
after a subcommand too, and both stay out of the input with `#[clifacet(skip)]`. Add
the application's own global options beside them as ordinary fields.

A dry run prints `Command::preview`: a `Planned` command answers with its plan — the
same plan a real run executes, so the two cannot disagree — and any other command
refuses, with exit code `2` and nothing run. With the `tui` feature, a run whose mode
resolves to the full-screen form is hosted by the runner too: the form generated from
the fields declared `form` is drawn on stderr, and a submission goes through the same
validation, command and output as the flags.

The reference application in `examples/simple` runs this way, and is the shortcut to
seeing each mode work:

```sh
cargo run -p clifacet-example                            # asks for what the flags left out
cargo run -p clifacet-example -- --name ada --repeat 2    # flags only
cargo run -p clifacet-example -- --interactive            # the dialog, explicitly
cargo run -p clifacet-example -- --tui                    # the full-screen form
```

Its own [README](../examples/simple/README.md) is the guided tour: the flags, the four
modes, the exit codes and which test proves each promise.

## What to read next

| Question | Page |
| --- | --- |
| Which mode runs, and who decides? | [Modes and the interaction policy](modes.md) |
| How does an existing `clap` command get validated input? | [The companion derive](derive.md) |
| How does dropping a file into a folder add a command? | [Discovery](derive.md#discovery-commands-with-no-list) |
| How do I draw my own screen without a second command? | [TUI composition](tui-composition.md) |
| How is any of this tested without a terminal? | [Testing](testing.md) |
| How do I work on the crate itself? | [Contributing](contributing.md) |

The reference material — every seam, the ownership rules and what is deliberately
not implemented — is in [Command core and its seams](command-core.md).
