//! The public seams of the reference application, driven through buffers.
//!
//! These tests never allocate a terminal: `example::main` takes its arguments and
//! its two streams as parameters, so a run is an ordinary function call. That is
//! the property the M1 exit criterion asks for — a domain-neutral command running
//! non-interactively and demonstrating its stream and exit behavior.

use clifacet::clap::Parser as _;
use clifacet::example::{self, Cli, Example, ExampleInput, MAX_REPEAT};
use clifacet::{Command, Failure, FailureKind};

/// Runs one invocation and returns its exit code, data stream and diagnostic stream.
#[allow(
    clippy::similar_names,
    reason = "`out` and `err` are the two streams every layer of this design names"
)]
fn invoke(args: &[&str]) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv = std::iter::once("example")
        .chain(args.iter().copied())
        .map(std::ffi::OsString::from);

    let code = example::main(argv, &mut out, &mut err);

    (
        code,
        String::from_utf8(out).expect("the example writes valid UTF-8"),
        String::from_utf8(err).expect("the example writes valid UTF-8"),
    )
}

#[test]
fn the_default_invocation_writes_data_on_the_stdout_side_only() {
    let (code, out, err) = invoke(&[]);

    assert_eq!(code, example::EXIT_SUCCESS);
    assert_eq!(out, "hello, world\n");
    assert_eq!(err, "", "a successful run has nothing to report");
}

#[rstest::rstest]
#[case(&["--name", "ada"], 1)]
#[case(&["--repeat", "3"], 3)]
#[case(&["--name", "ada", "--repeat", "8"], 8)]
fn the_repeat_flag_repeats_exactly_that_many_lines(#[case] args: &[&str], #[case] lines: usize) {
    let (code, out, err) = invoke(args);

    assert_eq!(code, example::EXIT_SUCCESS);
    assert_eq!(out.lines().count(), lines);
    assert_eq!(err, "");
}

#[rstest::rstest]
#[case(&["--repeat", "0"])]
#[case(&["--repeat", "9"])]
#[case(&["--name", "   "])]
fn invalid_input_is_a_validation_failure_with_no_data(#[case] args: &[&str]) {
    let (code, out, err) = invoke(args);

    assert_eq!(code, example::EXIT_VALIDATION_FAILED);
    assert_eq!(out, "", "validation writes no machine data");
    assert!(
        err.starts_with("example: "),
        "the application owns the diagnostic prefix, got {err:?}"
    );
}

#[test]
fn a_failing_operation_is_an_execution_failure_with_no_data() {
    let (code, out, err) = invoke(&["--explode"]);

    assert_eq!(
        code,
        example::EXIT_EXECUTION_FAILED,
        "an execution failure is not a validation failure"
    );
    assert_eq!(out, "");
    assert!(err.contains("--explode"), "got {err:?}");
}

#[test]
fn unbounded_values_are_rejected_by_the_library_not_by_the_parser() {
    // `u8` parsing would accept 255, so the range check has to be ours.
    let parsed = Cli::try_parse_from(["example", "--repeat", "255"]).expect("255 is a valid u8");
    let failure = ExampleInput::try_from(parsed).expect_err("255 is out of range");

    assert_eq!(failure.kind(), FailureKind::Validation);
    assert!(failure.message().contains(&MAX_REPEAT.to_string()));
}

#[test]
fn the_command_seam_runs_without_the_application_layer() {
    let input = ExampleInput {
        name: String::from("seam"),
        repeat: 2,
        explode: false,
    };

    let output = Example.execute(input).expect("a valid input runs");

    assert_eq!(output.lines, vec!["hello, seam", "hello, seam"]);
}

#[test]
fn the_command_seam_reports_execution_failures_as_that_kind() {
    let failure = Example
        .execute(ExampleInput {
            name: String::from("seam"),
            repeat: 1,
            explode: true,
        })
        .expect_err("--explode fails");

    assert_eq!(failure.kind(), FailureKind::Execution);
    assert_ne!(
        failure.kind(),
        FailureKind::Validation,
        "an execution failure must not look like validation to the application"
    );
}

#[test]
fn a_usage_error_is_reported_on_the_diagnostic_stream() {
    let (code, out, err) = invoke(&["--nope"]);

    assert_eq!(code, example::EXIT_USAGE);
    assert_eq!(out, "", "a usage error writes no machine data");
    assert!(err.contains("unexpected argument"), "got {err:?}");
}

#[test]
fn help_is_a_success_on_the_data_stream() {
    let (code, out, err) = invoke(&["--help"]);

    assert_eq!(
        code,
        example::EXIT_SUCCESS,
        "asking for help is not a failure"
    );
    assert!(out.contains("Usage:"), "got {out:?}");
    assert_eq!(err, "");
}

#[test]
fn the_version_flag_reports_the_package_version() {
    let (code, out, _err) = invoke(&["--version"]);

    assert_eq!(code, example::EXIT_SUCCESS);
    assert!(out.contains(clifacet::VERSION), "got {out:?}");
}

#[test]
fn a_closed_data_stream_becomes_an_execution_failure() {
    // A writer that cannot write has nowhere to report success, so the run must
    // not claim it succeeded.
    struct Broken;

    impl std::io::Write for Broken {
        fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut err = Vec::new();
    let code = example::run(
        Cli {
            name: None,
            repeat: 1,
            explode: false,
        },
        &mut Broken,
        &mut err,
    );

    assert_eq!(code, example::EXIT_EXECUTION_FAILED);
    assert!(String::from_utf8_lossy(&err).contains("cannot write the output"));
}

#[test]
fn a_failure_displays_only_its_message() {
    let failure = Failure::validation("--repeat must be between 1 and 8, got 0");

    assert_eq!(failure.to_string(), failure.message());
    assert_eq!(failure.kind(), FailureKind::Validation);
}
