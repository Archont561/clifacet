Feature: A full-screen form collects the same command input
  The form is a frontend, not a second command: what a person types reaches the
  same validation and the same handler, so the data does not depend on the mode.

  Scenario: A typed answer produces the same data as the flag
    Given the example's full-screen form
    When the user types ada and submits
    Then the data is what --name ada would produce
    And the exit code says the run succeeded

  Scenario: Leaving the form runs nothing
    Given the example's full-screen form
    When the user presses escape
    Then nothing is written to the data stream
    And the exit code says the run was cancelled
