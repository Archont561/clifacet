Feature: Prompting shares the command core
  An interactive run and a flag run reach the same command through the same
  validation. The frontend only fills the gaps a flag left empty, so the data a
  caller gets must not depend on how the value arrived.

  Scenario: A prompted answer produces the same data as the flag
    Given an application that may prompt
    When the user is asked for a name and answers ada
    Then the data is what --name ada would produce

  Scenario: A declined confirmation cancels the run
    Given an application that may prompt
    When the user asks to explode and declines the confirmation
    Then nothing is written to the data stream
    And the exit code says the run was cancelled
