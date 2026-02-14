Feature: Setting yak state
  Set a yak's workflow state: todo, wip, or done.
  The `yx start` command is a convenience alias for `yx state <name> wip`.

  Rule: Setting state explicitly changes the yak's state

    Example: Set a yak to wip state
      Given I have a clean git repository
      And I add the yak "get milk"
      When I set the state of "get milk" to "wip"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] get milk
        """

  Rule: Invalid states are rejected with a helpful error

    Example: Setting an invalid state shows an error
      Given I have a clean git repository
      And I add the yak "get milk"
      When I try to set the state of "get milk" to "invalid-state"
      Then the command should fail
      And the error should contain "Invalid state 'invalid-state'. Valid states are: todo, wip, done"

  Rule: Starting a yak is a convenience alias for setting state to wip

    Example: Starting a yak sets it to wip
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I start "Fix the bug"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] Fix the bug
        """
