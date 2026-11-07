Feature: A command tree reaches the leaf that was chosen
  A tree is declared where its commands live, and nothing in it knows how deep it
  sits. What a person sees is that the values typed at every level arrive together,
  that only the command they chose asks anything, and that only that command runs.

  Scenario: Every level's values reach the chosen leaf
    When the user runs "cast --verbose db --url replica migrate --steps 2"
    Then the application receives the root flag, the replica database and 2 steps

  Scenario: Only the chosen leaf asks its own question
    Given the tree runs at a terminal
    When the user runs "cast db reset" and confirms
    Then the only question asked was "Drop every table?"
    And the reset is forced

  Scenario: Running the tree runs only the chosen leaf
    When the user runs "cast greet --name ada" through the tree
    Then the tree answers "hello, ada"
    And no database command ran

  Scenario: Without a subcommand the application decides
    Given the tree runs at a terminal
    When the user runs "cast"
    Then the application receives no command
    And nothing was asked
