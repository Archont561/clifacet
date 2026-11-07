Feature: bl asks only what the command line left open
  A wrapper is mostly used from a pipe, so a question is the exception: it is asked
  when a person can answer it and the command line left it open, and otherwise the
  run either proceeds on what was said or refuses, naming what is missing.

  Scenario: A bare status filter is chosen from a list
    Given bl runs at a terminal
    When the user lists with a bare --status and picks In Progress
    Then the planned command filters by In Progress

  Scenario: A bare status filter nothing can ask about is refused
    Given bl runs without a terminal
    When the user lists with a bare --status
    Then the run is refused as incomplete, naming every status

  Scenario: --yes moves a task even without a terminal
    Given bl runs without a terminal
    When the user marks CLI-20 done with --yes
    Then the planned command moves CLI-20 to Done

  Scenario: A move nobody could confirm asks for --yes
    Given bl runs without a terminal
    When the user marks CLI-20 done
    Then the run is refused, saying to pass --yes

  Scenario: A declined move cancels the run
    Given bl runs at a terminal
    When the user marks CLI-20 done and declines
    Then nothing is planned and the run is cancelled
