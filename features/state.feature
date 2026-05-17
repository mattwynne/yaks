Feature: Setting yak state
  Set a yak's workflow state: todo, wip, blocked, or done.
  The `yx start` command is a guarded convenience for moving a ready yak to wip.

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
      And the error should contain "Invalid state 'invalid-state'. Valid states are: todo, wip, blocked, done"

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

  Rule: Done ancestors demote to todo when a child leaves done
    A parent cannot remain done if any child is not done.
    Reopening a child demotes done ancestors back to todo without
    implicitly promoting them to wip.

    Example: Child set from done to wip demotes done parent to todo
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      And I mark the yak "child" as done
      And I mark the yak "parent" as done
      When I set the state of "child" to "wip"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [wip] child
        """

    Example: Child set from done to todo demotes done parent to todo
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      And I mark the yak "child" as done
      And I mark the yak "parent" as done
      When I set the state of "child" to "todo"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """

    Example: Propagates through multiple ancestor levels
      Given I have a clean git repository
      And I add the yak "a"
      And I add the yak "b" under "a"
      And I add the yak "c" under "b"
      And I mark the yak "c" as done
      And I mark the yak "b" as done
      And I mark the yak "a" as done
      When I set the state of "c" to "wip"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] a
          - [todo] b
            - [wip] c
        """

    Example: Only affects ancestors in done state
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      And I mark the yak "child" as done
      When I set the state of "child" to "wip"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [wip] child
        """

    Example: Sibling state is irrelevant
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child-a" under "parent"
      And I add the yak "child-b" under "parent"
      And I mark the yak "child-a" as done
      And I mark the yak "child-b" as done
      And I mark the yak "parent" as done
      When I set the state of "child-a" to "wip"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [done] child-b
          - [wip] child-a
        """

  Rule: Setting state to blocked marks the yak as blocked

    Example: Set a yak to blocked state
      Given I have a clean git repository
      And I add the yak "get milk"
      When I set the state of "get milk" to "blocked"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [blocked] get milk
        """

  Rule: Starting a yak requires readiness

    Example: Starting a yak with active explicit blockers fails without a state event
      Given I have a clean git repository
      And I add the yak "blocked yak"
      And I add the yak "blocking yak"
      And I add blocker "blocking yak" to "blocked yak" with reason "waiting on it"
      When I try to start "blocked yak"
      Then the command should fail
      And the error should contain "cannot start 'blocked yak' - it is not ready"
      And the error should contain "blocked by blocking yak"
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] blocked yak
        - [todo] blocking yak
        """
      When I show the log
      Then the output should not include "started blocked yak"

    Example: Starting a parent with incomplete children fails without a state event
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      When I try to start "parent"
      Then the command should fail
      And the error should contain "cannot start 'parent' - it is not ready"
      And the error should contain "incomplete children: parent/child"
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """
      When I show the log
      Then the output should not include "started parent"

    Example: Starting a yak in a non-todo state fails
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      And I set the state of "Fix the bug" to "blocked"
      When I try to start "Fix the bug"
      Then the command should fail
      And the error should contain "cannot start 'Fix the bug' - it is not ready"
      And the error should contain "state is blocked"
