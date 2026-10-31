# TUI composition

The `tui` feature adds a full-screen frontend, and the components it draws with are
public. An application that wants a different screen builds its own out of the same
pieces rather than replacing the loop — that way the keys, the refusals and the
submission stay the shared ones (decision-4).

| Item | What it is |
| --- | --- |
| `tui::FormState`, `tui::FieldState` | the form: values, focus, the last refusal. No `ratatui` in sight |
| `tui::Key`, `tui::Outcome` | the keystrokes a form answers, and what it decided about one |
| `tui::Form`, `tui::Field` | `ratatui` widgets — the generated form is nothing but fields |
| `tui::CustomField` | one field, drawn by the application |
| `tui::app::run`, `run_on`, `run_with`, `run_on_with` | the loop: draw, read, submit |
| `tui::ratatui` | the frontend library at the version this crate was built against |

## Generate the form from the command

Which arguments a screen asks for is a design decision, so it is named, never guessed:

```rust
use clap::Parser;
use clifacet::tui::FormState;
use clifacet::Facet;

#[derive(Debug, Parser, Facet)]
#[facet(input = Input, bridge = build_input)]
struct Cli {
    /// Who to greet.
    #[arg(long, default_value = "world")]
    name: String,
    /// How many times to greet.
    #[arg(long, default_value_t = 1)]
    repeat: u8,
}

#[derive(Debug)]
struct Input {
    name: String,
    repeat: u8,
}

fn build_input(cli: Cli) -> Result<Input, clifacet::Failure> {
    Ok(Input { name: cli.name.clone(), repeat: cli.repeat })
}

let state = FormState::generated(&Cli::command(), &["name", "repeat"])
    .expect("the command has both arguments");

assert_eq!(state.fields().len(), 2);
```

The label comes from the argument's help and the initial value from its default. A
name the command does not have is an error, not an empty form: it is a mistake in the
application, not something a user can fix.

## The two escape hatches

Exactly two things can be replaced, and both are explicit:

```rust
use clifacet::tui::ratatui::buffer::Buffer;
use clifacet::tui::ratatui::layout::Rect;
use clifacet::tui::ratatui::widgets::{Paragraph, Widget};
use clifacet::tui::{CustomField, FieldState};

/// One field, drawn by the application: the value is masked.
struct Masked;

impl CustomField for Masked {
    fn draw(&self, field: &FieldState, focused: bool, area: Rect, buffer: &mut Buffer) {
        let marker = if focused { "> " } else { "  " };
        let hidden = "*".repeat(field.value().chars().count());
        Paragraph::new(format!("{marker}{}: {hidden}", field.label())).render(area, buffer);
    }
}

let _hatch = Masked;
```

The second hatch is the whole screen: `tui::app::run_with` takes the drawing as a
parameter, and `run_on_with` is the same hatch with the alternate screen, so a custom
screen does not have to reimplement entering and leaving it.

Neither hatch is a registry and neither can change behaviour. `run_with` replaces the
drawing and nothing else: the same keys, the same refusals, the same submission
through `clap` and the same handler, which is why a custom screen reaches the same
validation as the generated one.

## Submission goes through `clap`

The answers become arguments, `clap` parses them, and validation plus the command run
exactly as they would for flags. A rejected answer keeps the form open and shows the
same words the flags would print, so a typo costs an edit rather than the run.

## Testing a screen without a terminal

`tui::app::run` takes a `ratatui` backend and an iterator of `Key`s. Draw into a
`TestBackend` and pass a script instead of a keyboard:

```rust
use clifacet::tui::ratatui::backend::TestBackend;
use clifacet::tui::ratatui::Terminal;
use clifacet::tui::{FormState, Key};

let state = FormState::new(vec![clifacet::tui::FieldState::new("name", "Who?", "world")]);
let keys = [Key::Enter];

let mut terminal = Terminal::new(TestBackend::new(60, 10)).expect("a test screen");
let outcome = clifacet::tui::app::run_with(
    &mut terminal,
    &mut state.clone(),
    keys,
    clifacet::tui::draw_form,
    |_state| Ok(vec![String::from("hello, world")]),
);

assert!(outcome.is_ok(), "the form submits the answer it was given");
```

What no test can reach is the process's own terminal: the size probe, the raw-mode
restore and the live event stream. Everything they wrap is covered, which is why the
coverage report has five uncovered lines rather than a list of them.

The runnable versions of both hatches are in
`crates/clifacet-example/examples/tui_custom_screen.rs`.
