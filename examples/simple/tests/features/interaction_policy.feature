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

  Scenario: An application that prefers prompts asks instead of drawing a screen
    A policy states both what it permits and what it wants. Prompts are what this
    application wants, so an unadorned run asks for the value the flags left out
    rather than taking over the terminal with a form.

    Given a terminal that could host a full-screen form
    When the user runs with no flags
    Then the answer becomes the data
    And no screen is drawn

  Scenario: --yes answers confirmations and leaves the mode alone
    Given an application that runs non-interactively
    When the user runs with --yes
    Then the greeting is data, not a question
    And the exit code says the run succeeded
