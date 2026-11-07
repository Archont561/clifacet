Feature: A run records where each value came from, never the value
  Where a value came from is read from the parse and from the questions the session
  asked, so an application does not have to keep the record by hand. A default is not
  something the user typed, an environment variable is not a flag, an answer is not a
  flag either, and a value nobody supplied has no source at all.

  Scenario: Typed, defaulted and environment values are told apart
    Given the run has no terminal
    When the user types "--name ada"
    Then "name" came from "argv"
    And "repeat" came from "default"
    And "path" came from "environment"

  Scenario: A value nobody supplied has no source
    Given the run has no terminal
    When the user types nothing
    Then "name" has no source
    And nothing was asked

  Scenario: An answer to a question is recorded as asked, not typed
    Given the run is at a terminal
    When the user types nothing and answers "ada"
    Then "name" came from "prompt"
    And "repeat" came from "default"
    And the record does not contain "ada"
