Feature: The mode that collected the input changes nothing else
  A flag run, a scripted prompt and a full-screen form are the same command. The
  parity harness is the public way to say so with an assertion rather than a promise.

  Scenario: Every mode reaches the command with the same answer
    Given the example command, recorded through the parity harness
    When ada is supplied through the flags, the scripted prompt and the form
    Then all three modes agree on the data, the exit code and the recorded input

  Scenario: A rejected answer runs nothing
    Given the example command, recorded through the parity harness
    When an out-of-range repeat is supplied through the flags and the scripted prompt
    Then neither run reaches the command
    And both report the same failure and exit code
