use super::VERSION;

#[test]
fn version_matches_package_metadata() {
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(super::version(), VERSION);
}

#[test]
fn reexported_macro_reports_matching_workspace_version() {
    assert_eq!(crate::version!(), VERSION);
}

mod failure {
    use crate::{Failure, FailureKind};

    #[test]
    fn the_two_kinds_are_distinguishable() {
        let validation = Failure::validation("bad input");
        let execution = Failure::execution("bad input");

        assert_eq!(validation.kind(), FailureKind::Validation);
        assert_eq!(execution.kind(), FailureKind::Execution);
        assert_ne!(validation.kind(), execution.kind());
    }

    #[test]
    fn the_message_is_recoverable_and_displayed_verbatim() {
        let failure = Failure::execution("the operation failed");

        assert_eq!(failure.message(), "the operation failed");
        assert_eq!(failure.to_string(), "the operation failed");
    }

    #[test]
    fn a_failure_is_a_std_error() {
        let failure = Failure::validation("bad input");
        let as_error: &dyn std::error::Error = &failure;

        assert_eq!(as_error.to_string(), "bad input");
    }
}

mod example {
    use crate::Command;
    use crate::example::{Example, ExampleInput, Level, MAX_REPEAT};
    use crate::failure::FailureKind;

    fn input(name: &str, repeat: u8) -> ExampleInput {
        ExampleInput {
            name: name.to_owned(),
            repeat,
            explode: false,
            level: Level::Basic,
        }
    }

    #[test]
    fn execution_produces_one_greeting_per_repeat() {
        let output = Example
            .execute(input("ada", 3))
            .expect("a valid input runs");

        assert_eq!(output.lines.len(), 3);
        assert_eq!(output.to_string(), "hello, ada\nhello, ada\nhello, ada");
    }

    #[test]
    fn the_upper_bound_is_inclusive() {
        let output = Example
            .execute(input("bounds", MAX_REPEAT))
            .expect("the maximum is valid");

        assert_eq!(output.lines.len(), usize::from(MAX_REPEAT));
    }

    #[test]
    fn explode_reports_an_execution_failure() {
        let failure = Example
            .execute(ExampleInput {
                name: String::from("ada"),
                repeat: 1,
                explode: true,
                level: Level::Basic,
            })
            .expect_err("--explode fails");

        assert_eq!(failure.kind(), FailureKind::Execution);
    }
}

mod interaction {
    use crate::example::{
        self, EXIT_CANCELLED, EXIT_EXECUTION_FAILED, EXIT_VALIDATION_FAILED, exit_code,
    };
    use crate::failure::Failure;
    use crate::policy::{Mode, Policy};

    /// Every category gets its own number, so a caller can tell them apart without
    /// reading the message.
    #[test]
    fn each_failure_kind_has_its_own_exit_code() {
        let cancelled = exit_code(&Failure::cancelled("changed their mind"));
        let execution = exit_code(&Failure::execution("broke"));
        let validation = exit_code(&Failure::validation("bad input"));

        assert_eq!(cancelled, EXIT_CANCELLED);
        assert_eq!(execution, EXIT_EXECUTION_FAILED);
        assert_eq!(validation, EXIT_VALIDATION_FAILED);
        assert_ne!(
            cancelled, validation,
            "a cancel is not a mistake in the invocation"
        );
        assert_ne!(cancelled, execution);
        assert_ne!(
            cancelled,
            example::EXIT_SUCCESS,
            "nothing ran, so the process did not succeed"
        );
    }

    #[test]
    fn a_policy_reports_the_ceiling_it_was_given() {
        assert_eq!(Policy::non_interactive().ceiling(), Mode::NonInteractive);
        assert_eq!(
            Policy::up_to(Mode::Interactive).ceiling(),
            Mode::Interactive
        );
        assert_eq!(Policy::up_to(Mode::Tui).ceiling(), Mode::Tui);

        // The default is the least interactive policy, so a forgotten ceiling can
        // never prompt.
        assert_eq!(Policy::default().ceiling(), Mode::NonInteractive);
    }
}
