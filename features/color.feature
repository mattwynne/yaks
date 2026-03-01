Feature: Color output

  The yx CLI respects the NO_COLOR convention (https://no-color.org/)
  and suppresses ANSI color codes when stdout is not a terminal.

  Rule: Color output respects NO_COLOR and terminal detection

    @fullstack
    Example: NO_COLOR suppresses ANSI escape codes
      Given I have a clean git repository
      And I add the yak "my yak"
      When I list the yaks with NO_COLOR set
      Then the output should not contain escape codes

    @fullstack
    Example: Non-TTY output suppresses ANSI escape codes
      Given I have a clean git repository
      And I add the yak "my yak"
      When I list the yaks piped through cat
      Then the output should not contain escape codes
