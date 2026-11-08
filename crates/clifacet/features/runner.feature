Feature: The runner owns an application's pipeline
  An application hands its command line to the runner, which parses, asks, validates,
  runs or previews, writes the output and answers with an exit code. Whatever stops a
  run early — help, a dry run, a declined question — stops it before anything changes.

  Background:
    Given an application whose save command records every file it writes

  Scenario: Asking for help runs nothing
    When the user runs "save notes.txt --help"
    Then the help is written on the data stream
    And the run exits with the success code
    And no file was written

  Scenario: A dry run shows what would change and changes nothing
    When the user runs "save notes.txt --dry-run"
    Then the data stream reads "write notes.txt"
    And the run exits with the success code
    And no file was written

  Scenario: A declined question cancels the run
    Given a terminal where the user declines every confirmation
    When the user runs "save notes.txt --force"
    Then the run exits with the cancellation code
    And nothing is written on the data stream
    And no file was written

  Scenario: A real run writes what the dry run showed
    When the user runs "save notes.txt"
    Then the data stream reads "saved notes.txt"
    And the run exits with the success code
    And the file written was "notes.txt"
