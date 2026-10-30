//! decision-3's interaction policy, as tables rather than prose.
//!
//! The precedence is fixed: an explicit request, then the application's policy,
//! then what the environment can offer. Each row below pins one step of that, and
//! the shape of the API is deliberately small enough that every combination is
//! enumerable — a resolver with states nobody can name is a resolver nobody can
//! test.

use std::cell::Cell;

use clifacet::policy::{self, Capabilities, Capability, Mode, Policy, Refusal, Request};
use clifacet::{Command, Failure, FailureKind};
use rstest::rstest;

/// An explicit request for one mode, with no confirmation preference.
fn asking_for(mode: Mode) -> Request {
    Request::mode(mode)
}

/// The refusal an application's policy gives for a mode above its ceiling.
fn forbidden(requested: Mode, ceiling: Mode) -> Refusal {
    Refusal::Forbidden { requested, ceiling }
}

/// The refusal an environment gives for a mode it cannot host.
fn unavailable(requested: Mode, missing: Capability) -> Refusal {
    Refusal::Unavailable { requested, missing }
}

#[rstest]
#[case::explicit_tui(
    Request::mode(Mode::Tui),
    Policy::up_to(Mode::Tui),
    Capabilities::terminal_with_tui(),
    Ok(Mode::Tui)
)]
#[case::explicit_interactive(
    Request::mode(Mode::Interactive),
    Policy::up_to(Mode::Interactive),
    Capabilities::terminal(),
    Ok(Mode::Interactive)
)]
#[case::explicit_non_interactive(
    Request::mode(Mode::NonInteractive),
    Policy::up_to(Mode::Tui),
    Capabilities::terminal_with_tui(),
    Ok(Mode::NonInteractive)
)]
// An explicit request beats the policy's preference, but not its permission.
#[case::request_beyond_the_ceiling(
    Request::mode(Mode::Tui),
    Policy::up_to(Mode::Interactive),
    Capabilities::terminal_with_tui(),
    Err(forbidden(Mode::Tui, Mode::Interactive))
)]
#[case::request_needs_a_terminal(
    Request::mode(Mode::Interactive),
    Policy::up_to(Mode::Interactive),
    Capabilities::headless(),
    Err(unavailable(Mode::Interactive, Capability::Terminal))
)]
#[case::request_needs_tui_support(
    Request::mode(Mode::Tui),
    Policy::up_to(Mode::Tui),
    Capabilities::terminal(),
    Err(unavailable(Mode::Tui, Capability::Tui))
)]
// With no request, the policy picks the most interactive mode it allows, and
// capability can only take that away — never add more interaction than permitted.
#[case::policy_prefers_tui(
    Request::none(),
    Policy::up_to(Mode::Tui),
    Capabilities::terminal_with_tui(),
    Ok(Mode::Tui)
)]
#[case::policy_falls_back_to_prompts(
    Request::none(),
    Policy::up_to(Mode::Tui),
    Capabilities::terminal(),
    Ok(Mode::Interactive)
)]
#[case::headless_falls_back_to_flags(
    Request::none(),
    Policy::up_to(Mode::Tui),
    Capabilities::headless(),
    Ok(Mode::NonInteractive)
)]
// A capability on its own is never consent: without a policy that permits it, a
// terminal does not make the run interactive (decision-3).
#[case::a_terminal_is_not_consent(
    Request::none(),
    Policy::non_interactive(),
    Capabilities::terminal_with_tui(),
    Ok(Mode::NonInteractive)
)]
fn resolution_follows_request_then_policy_then_capability(
    #[case] request: Request,
    #[case] policy: Policy,
    #[case] capabilities: Capabilities,
    #[case] expected: Result<Mode, Refusal>,
) {
    let resolved = policy
        .resolve(request, capabilities)
        .map(clifacet::policy::Plan::mode);

    assert_eq!(resolved, expected);
}

#[rstest]
#[case::flags_without_yes(Mode::Interactive, false, Mode::Interactive)]
#[case::prompts_with_yes(Mode::Interactive, true, Mode::Interactive)]
#[case::flags_with_yes(Mode::NonInteractive, true, Mode::NonInteractive)]
#[case::no_request_with_yes(Mode::NonInteractive, true, Mode::NonInteractive)]
fn yes_changes_confirmations_and_nothing_else(
    #[case] request: Mode,
    #[case] assume_yes: bool,
    #[case] expected: Mode,
) {
    let policy = Policy::up_to(Mode::Interactive);
    let plan = policy
        .resolve(
            Request::mode(request).assuming_yes(assume_yes),
            Capabilities::terminal(),
        )
        .expect("this policy permits the request");

    assert_eq!(plan.mode(), expected, "--yes is not a mode");
    assert_eq!(
        plan.assumes_yes(),
        assume_yes,
        "--yes answers confirmations instead of asking"
    );
}

#[rstest]
#[case::flags(Mode::NonInteractive)]
#[case::prompts(Mode::Interactive)]
#[case::tui(Mode::Tui)]
fn only_the_interactive_modes_ask_the_user(#[case] mode: Mode) {
    assert_eq!(
        mode.asks_the_user(),
        mode != Mode::NonInteractive,
        "{mode:?}"
    );
}

#[test]
fn a_forbidden_mode_never_constructs_a_frontend() {
    let constructed = Cell::new(0);
    let build = || constructed.set(constructed.get() + 1);

    // A refused request produces no plan at all, so there is nothing a frontend
    // could be attached to: an application cannot prompt in CI by asking nicely.
    let refused =
        Policy::non_interactive().resolve(asking_for(Mode::Interactive), Capabilities::terminal());
    assert!(
        refused.is_err(),
        "a non-interactive application refuses prompts"
    );
    assert!(
        refused.is_err_and(|refusal| refusal == forbidden(Mode::Interactive, Mode::NonInteractive))
    );
    assert_eq!(constructed.get(), 0, "nothing was constructed");

    // A permitted run that asks nothing still yields no frontend.
    let flags = Policy::non_interactive()
        .resolve(Request::none(), Capabilities::headless())
        .expect("the default resolves");
    assert_eq!(flags.with_frontend(build), None);
    assert_eq!(constructed.get(), 0, "still nothing was constructed");

    // The interactive plan is the only one that builds anything, exactly once.
    let interactive = Policy::up_to(Mode::Interactive)
        .resolve(Request::none(), Capabilities::terminal())
        .expect("prompts are allowed here");
    assert_eq!(interactive.with_frontend(build), Some(()));
    assert_eq!(constructed.get(), 1, "it was constructed exactly once");
}

#[test]
fn a_cancelled_run_is_not_a_validation_failure() {
    let cancelled = Failure::cancelled("the user cancelled the prompt");

    assert_eq!(cancelled.kind(), FailureKind::Cancelled);
    assert_ne!(
        cancelled.kind(),
        FailureKind::Validation,
        "cancellation is a decision, not bad input"
    );
    assert_ne!(cancelled.kind(), FailureKind::Execution);
    assert_eq!(cancelled.message(), "the user cancelled the prompt");
}

/// Counts how often it was asked to run, without touching a stream.
#[derive(Default)]
struct Counting {
    runs: Cell<usize>,
}

impl Command for Counting {
    type Input = ();
    type Output = usize;

    fn execute(&self, (): ()) -> Result<Self::Output, Failure> {
        self.runs.set(self.runs.get() + 1);
        Ok(self.runs.get())
    }
}

#[rstest]
#[case::refused_mode(
    Request::mode(Mode::Tui),
    Policy::up_to(Mode::Interactive),
    Capabilities::terminal(),
    0
)]
#[case::unavailable_mode(
    Request::mode(Mode::Interactive),
    Policy::up_to(Mode::Interactive),
    Capabilities::headless(),
    0
)]
#[case::allowed_mode(
    Request::mode(Mode::Interactive),
    Policy::up_to(Mode::Interactive),
    Capabilities::terminal(),
    1
)]
#[case::default_mode(
    Request::none(),
    Policy::non_interactive(),
    Capabilities::terminal(),
    1
)]
fn a_refused_request_never_executes_the_handler(
    #[case] request: Request,
    #[case] policy: Policy,
    #[case] capabilities: Capabilities,
    #[case] expected_runs: usize,
) {
    let command = Counting::default();

    let outcome = policy::invoke(&command, (), request, policy, capabilities);

    assert_eq!(command.runs.get(), expected_runs, "handler invocations");
    if expected_runs == 0 {
        let failure = outcome.expect_err("a refused request reports a failure");
        assert_eq!(failure.kind(), FailureKind::Validation);
        assert!(
            failure.message().contains("non-interactive")
                || failure.message().contains("terminal")
                || failure.message().contains("TUI"),
            "the message explains the refusal: {}",
            failure.message()
        );
    } else {
        assert_eq!(outcome.expect("an allowed request runs"), expected_runs);
    }
}

#[rstest]
#[case::forbidden(forbidden(Mode::Tui, Mode::NonInteractive), "TUI", "non-interactive")]
#[case::no_terminal(
    unavailable(Mode::Interactive, Capability::Terminal),
    "interactive",
    "terminal"
)]
#[case::no_tui_support(unavailable(Mode::Tui, Capability::Tui), "TUI", "TUI")]
fn a_refusal_says_what_was_asked_for_and_why(
    #[case] refusal: Refusal,
    #[case] asked_for: &str,
    #[case] reason: &str,
) {
    let message = refusal.message();

    assert!(message.contains(asked_for), "{message}");
    assert!(message.contains(reason), "{message}");
}
