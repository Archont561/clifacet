Feature: Nothing runs unless the interaction mode is allowed
  A command that cannot prompt still has to answer a request for prompts. It
  refuses, says why, and writes nothing to the data stream — no partial run, no
  frontend, no terminal.

  Scenario: A request for a TUI is refused before anything runs
    Given an application that runs non-interactively
    When the user asks for a TUI
    Then nothing is written to the data stream
    And the diagnostic names the TUI
    And the exit code says nothing ran

  Scenario: --yes answers confirmations and leaves the mode alone
    Given an application that runs non-interactively
    When the user runs with --yes
    Then the greeting is data, not a question
    And the exit code says the run succeeded
