Feature: Secrets stay out of what the framework shows
  A supplied credential is redacted wherever the framework speaks: a diagnostic it
  writes, a preview it draws, a record it keeps. Provenance says where a value came
  from and holds no value at all.

  Scenario: A rejected credential is redacted
    Given a credential the example refuses
    When the example is run with it
    Then the diagnostic explains the rejection
    And the credential value appears nowhere

  Scenario: Provenance names the source of an answer
    Given a run whose answer came from a prompt
    When the run's provenance is recorded
    Then it names the answer and its source
    And it holds no value at all

  Scenario: A prompted secret is recorded, never shown
    Given a session that may prompt for a token
    When the user types the token where it is asked for
    Then the token is reachable only through Secret
    And provenance records it as a secret from the prompt

  Scenario: A frontend that cannot hide input does not ask for a secret
    Given a line-reading terminal
    When a token is asked for through it
    Then it refuses before the question is shown or anything is read
