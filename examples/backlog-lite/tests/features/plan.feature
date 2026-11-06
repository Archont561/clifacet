Feature: A dry run is the plan a real run executes
  bl is a planned command: a request becomes a plan, --dry-run prints it, and a real
  run executes that same plan. The preview is therefore the run, described, rather
  than a second code path that has to be kept in step with the first.

  Scenario: A dry run prints the plan, and only the plan
    Given bl runs without a terminal
    When the user previews creating a task titled "Ship it" with one criterion
    Then the output is exactly the plan bl would execute
    And the run succeeded without asking anything
