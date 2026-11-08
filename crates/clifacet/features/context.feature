Feature: An application can give its commands a context
  What a command depends on — a store, a client, a clock — is not something the user
  typed, so it does not belong in the input. The runner builds it after the input is
  valid and on first use, so nothing that stops a run early pays for it, and a question
  can read it to offer what already exists.

  Background:
    Given an application whose done command marks a task in a store

  Scenario: A real run marks the task the user named
    When the user runs "done --task a"
    Then the data stream reads "a is done"
    And the run exits with the success code
    And the run marked "a" done

  Scenario: A dry run previews with the context and changes nothing
    When the user runs "done --task b --dry-run"
    Then the data stream reads "mark b done"
    And the run exits with the success code
    And no task was marked done

  Scenario: Asking for help builds no context
    When the user runs "done --help"
    Then the help is written on the data stream
    And the run exits with the success code
    And the context was never built

  Scenario: A rejected value builds no context
    When the user runs "done --task a b"
    Then the run exits with the usage code
    And the context was never built

  Scenario: A declined question builds no context
    Given a terminal where the user declines every confirmation
    When the user runs "done --task a"
    Then the run exits with the cancellation code
    And the context was never built
    And no task was marked done

  Scenario: A context that cannot be built stops the run
    Given a store that cannot be opened
    When the user runs "done --task a"
    Then the run exits with the failure code
    And the diagnostics read "cannot prepare the application: the store is locked"
    And no task was marked done

  Scenario: A question offers the store's records as choices
    Given a terminal where the user picks "b"
    When the user runs "done"
    Then the question asked was "Which task?"
    And the data stream reads "b is done"
    And the context was built once
