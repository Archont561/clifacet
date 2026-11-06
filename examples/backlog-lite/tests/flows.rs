//! What the wrapper promises, asserted against the argument vector it would run.

use clifacet::failure::Failure;
use clifacet::policy::{Capabilities, Mode, Policy, Request as ModeRequest};
use clifacet::prompt::{Answer, Scripted, Session};
use clifacet_backlog_lite::{Cli, Request, Status, Verb, plan_of as plan};

fn argv(cli: Cli) -> Vec<String> {
    plan(Request::try_from(cli).expect("valid")).argv().to_vec()
}

fn cli(arguments: &[&str]) -> Cli {
    use clap::Parser as _;
    Cli::try_parse_from(std::iter::once("bl").chain(arguments.iter().copied()))
        .expect("the wrapper parses its own command line")
}

#[test]
fn no_subcommand_is_the_common_flow() {
    assert_eq!(argv(cli(&[])), ["backlog", "task", "list"]);
}

#[test]
fn list_forwards_only_what_was_asked_for() {
    assert_eq!(
        argv(cli(&[
            "list",
            "--ready",
            "--limit",
            "5",
            "--status",
            "in progress"
        ])),
        [
            "backlog",
            "task",
            "list",
            "--status",
            "In Progress",
            "--ready",
            "--limit",
            "5"
        ]
    );
}

#[test]
fn json_is_one_flag_instead_of_one_per_subcommand() {
    assert_eq!(
        argv(cli(&["--json", "show", "CLI-1"])),
        ["backlog", "task", "view", "CLI-1", "--json"]
    );
}

#[test]
fn show_is_plain_by_default_because_a_wrapper_is_a_pipe() {
    assert_eq!(
        argv(cli(&["show", "CLI-1"])),
        ["backlog", "task", "view", "CLI-1", "--plain"]
    );
}

#[test]
fn new_forwards_each_acceptance_criterion_separately() {
    assert_eq!(
        argv(cli(&[
            "new",
            "Ship it",
            "--ac",
            "one",
            "--ac",
            "two",
            "--milestone",
            "m-5"
        ])),
        [
            "backlog",
            "task",
            "create",
            "Ship it",
            "--ac",
            "one",
            "--ac",
            "two",
            "--milestone",
            "m-5",
        ]
    );
}

#[test]
fn start_and_done_are_shortcuts_for_the_edit_no_one_remembers() {
    assert_eq!(
        argv(cli(&["start", "CLI-20"])),
        [
            "backlog",
            "task",
            "edit",
            "CLI-20",
            "--status",
            "In Progress"
        ]
    );
    assert_eq!(
        argv(cli(&["done", "CLI-20"])),
        ["backlog", "task", "edit", "CLI-20", "--status", "Done"]
    );
}

#[test]
fn a_status_is_validated_with_the_words_the_tool_uses() {
    use clap::Parser as _;
    let error = Cli::try_parse_from(["bl", "list", "--status", "in-progres"]).expect_err("rejects");
    let text = error.to_string();
    assert!(text.contains("To Do"), "{text}");
    assert!(
        text.contains("In Progress") && text.contains("Done"),
        "{text}"
    );
}

#[test]
fn dry_run_prints_a_line_that_could_be_pasted_back() {
    let planned =
        plan(Request::try_from(cli(&["new", "Two words", "--ac", "a b"])).expect("valid"));
    assert_eq!(
        planned.shell_line(),
        "backlog task create 'Two words' --ac 'a b'"
    );
    assert!(
        planned.explain().contains("Two words"),
        "{}",
        planned.explain()
    );
    assert!(
        planned.explain().contains("\u{201c}"),
        "{}",
        planned.explain()
    );
}

#[test]
fn a_missing_title_is_asked_for_when_something_can_answer() {
    let mut session = Session::start(interactive(), || {
        Box::new(Scripted::new([Answer::Text(String::from("ship it"))]))
    });
    let args = cli(&["new"]);
    let Some(Verb::New(new)) = args.command else {
        panic!("expected the new subcommand");
    };
    let request = clifacet_backlog_lite::collect_new(new, &mut session).expect("collects");
    assert_eq!(plan(request).argv()[3], "ship it");
}

#[test]
fn a_missing_title_without_a_terminal_fails_instead_of_guessing() {
    let mut session = Session::start(headless(), || Box::new(Scripted::default()));
    let cli = cli(&["new"]);
    let Some(Verb::New(new)) = cli.command else {
        panic!("expected the new subcommand");
    };
    let failure = clifacet_backlog_lite::collect_new(new, &mut session).expect_err("refuses");
    assert_eq!(failure.kind(), clifacet::FailureKind::Validation);
}

#[test]
fn a_state_change_is_confirmed_and_a_declined_one_runs_nothing() {
    let mut session = Session::start(interactive(), || Box::new(Scripted::new([Answer::No])));
    let failure = clifacet_backlog_lite::confirm_status("CLI-20", Status::Done, &mut session)
        .expect_err("declined");
    assert_eq!(failure.kind(), clifacet::FailureKind::Cancelled);
    assert!(matches!(failure, Failure { .. }));
}

#[test]
fn yes_answers_the_confirmation_without_asking() {
    let mut session = Session::start(interactive_with_yes(), || Box::new(Scripted::default()));
    clifacet_backlog_lite::confirm_status("CLI-20", Status::Done, &mut session).expect("confirmed");
}

/// `--yes` is the answer a pipe gives in advance, so a run with no terminal that passed
/// it must proceed rather than be told to pass the flag it already passed.
#[test]
fn yes_settles_the_confirmation_even_when_nothing_can_ask() {
    let mut session = Session::start(headless_with_yes(), || Box::new(Scripted::default()));
    clifacet_backlog_lite::confirm_status("CLI-20", Status::Done, &mut session)
        .expect("--yes said so in advance");
}

#[test]
fn a_state_change_nothing_can_confirm_asks_for_yes_instead_of_declining() {
    let mut session = Session::start(headless(), || Box::new(Scripted::default()));
    let failure = clifacet_backlog_lite::confirm_status("CLI-20", Status::Done, &mut session)
        .expect_err("refuses");
    assert_eq!(failure.kind(), clifacet::FailureKind::Validation);
    assert!(failure.message().contains("--yes"), "{}", failure.message());
}

fn interactive() -> clifacet::policy::Plan {
    Policy::up_to(Mode::Interactive)
        .resolve(
            ModeRequest::mode(Mode::Interactive),
            Capabilities::terminal(),
        )
        .expect("resolves")
}

fn interactive_with_yes() -> clifacet::policy::Plan {
    Policy::up_to(Mode::Interactive)
        .resolve(
            ModeRequest::mode(Mode::Interactive).assuming_yes(true),
            Capabilities::terminal(),
        )
        .expect("resolves")
}

fn headless_with_yes() -> clifacet::policy::Plan {
    Policy::up_to(Mode::Interactive)
        .resolve(
            ModeRequest::none().assuming_yes(true),
            Capabilities::headless(),
        )
        .expect("resolves")
}

fn headless() -> clifacet::policy::Plan {
    Policy::non_interactive()
        .resolve(ModeRequest::none(), Capabilities::headless())
        .expect("resolves")
}
