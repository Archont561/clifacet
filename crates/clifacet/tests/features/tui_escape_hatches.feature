Feature: Composition without a separate command core
  A screen of an application's own is a drawing decision, not a second command:
  it collects into the same draft, submits through the same validation and reaches
  the same handler as the generated form.

  Scenario: A custom field draws inside the generated form
    Given a form with a masked field for repeat
    When the form is drawn
    Then the masked answer is shown
    And the other fields are drawn as usual

  Scenario: A custom screen submits to the same command
    Given a screen of my own
    When the user types ada and submits
    Then the data is what --name ada would produce
    And the exit code says the run succeeded
