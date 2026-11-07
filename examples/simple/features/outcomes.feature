Feature: Every outcome is distinct and every stream stays clean
  The data stream carries only what a caller asked for. Everything a person sees —
  a question, a screen, a diagnostic — goes to the diagnostic stream. The four
  outcomes of a run each cost their own exit code.

  Scenario: A successful run writes data and nothing else
    Given the example can run without a terminal
    When the example greets ada
    Then the data stream is exactly the greeting
    And the diagnostic stream is empty
    And the exit code is the success code

  Scenario: A question is never data
    Given the example may ask a question
    When the answer is ada
    Then the data stream is exactly the greeting
    And the question is on the diagnostic stream

  Scenario: A failure writes a diagnostic and no data
    Given the example can run without a terminal
    When the repeat count is out of range
    Then the data stream is empty
    And the exit code is the validation code
