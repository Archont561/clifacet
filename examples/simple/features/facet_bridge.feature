Feature: Prove an existing clap command through Clifacet
  An application that already has a clap grammar should gain validated typed
  input without a second grammar, without a terminal, and without changing how
  its flags, aliases or value parsers behave.

  Scenario: An alias resolves to the same typed input
    Given an application whose name flag also answers to nickname
    When the user runs the command with --nickname and ada
    Then the typed input carries the name "ada"
    And the command runs without a terminal

  Scenario: A field declared once carries its own grammar
    Given a command whose arguments are declared with one attribute per field
    When the user asks for help
    Then the help shows the short flag, the value name and the field's doc comment
    And the same declaration parses a value through its alias

  Scenario: A field declared once is asked for and normalised by the same declaration
    Given a command whose name is declared with a question, a default and trimming
    When the user runs it at a terminal without --name and answers "  grace "
    Then the declared input carries the name "grace"
    And the user was asked "Who should I greet?" once

  Scenario: Without a terminal the declared default fills in and nothing is asked
    Given a command whose name is declared with a question, a default and trimming
    When the user runs it without a terminal and without --name
    Then the declared input carries the name "world"
    And the user was asked nothing

  Scenario: The reference application checks its input after each field's own rule
    Given the reference application, declared one attribute per field
    When the user runs it with a blank name and too many repeats
    Then validation fails with "--name must not be blank"
    And a name that is only padded still meets the repeat check "--repeat must be between 1 and 8, got 9"

  Scenario: The reference application's form is generated from its declarations
    Given the reference application, declared one attribute per field
    When its form is built
    Then the form asks for "name", "repeat" and "level", in that order

  Scenario: An input named through a trait the commands share is built from the declarations
    Given a command whose input is written "<Greet as Family>::Input", a generic type
    When the user runs that command with --name "ada"
    Then the generic input carries the name "ada"
    And without --name the declared default fills it in as "world"
