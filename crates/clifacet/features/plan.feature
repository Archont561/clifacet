Feature: A planned command's preview is the run it describes
  A command that plans before it runs can be previewed and then run, and the two
  cannot disagree: the library runs a planned command by executing its plan, so there
  is no second way to run one. A request the plan refuses runs nothing at all.

  Scenario: A preview lists every step and changes nothing
    Given a deploy command that plans before it runs
    When the application previews deploying "web" and "db"
    Then the preview lists "deploy web" then "deploy db"
    And nothing was deployed

  Scenario: A real run does exactly what the preview listed
    Given a deploy command that plans before it runs
    When the application previews deploying "web" and "db"
    And the application runs deploying "web" and "db"
    Then what was deployed is what the preview listed
    And the run answers 2

  Scenario: A request refused while planning runs nothing
    Given a deploy command that plans before it runs
    When the application runs deploying nothing
    Then the run fails as invalid input
    And nothing was deployed
