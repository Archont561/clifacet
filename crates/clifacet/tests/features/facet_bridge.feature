Feature: Prove an existing clap command through Clifacet
  An application that already has a clap grammar should gain validated typed
  input without a second grammar, without a terminal, and without changing how
  its flags, aliases or value parsers behave.

  Scenario: An alias resolves to the same typed input
    Given an application whose name flag also answers to nickname
    When the user runs the command with --nickname and ada
    Then the typed input carries the name "ada"
    And the command runs without a terminal
