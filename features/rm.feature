Feature: Remove yaks
  Delete yaks that are no longer needed. Removes only the specified
  yak, not its parent or children. Uses fuzzy matching for name
  resolution. Use rm for specific yaks, prune for bulk cleanup.

  Rule: Removing a yak deletes it from the list

    Example: Remove one of two yaks
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      And I add the yak "Write docs"
      When I remove the yak "Fix the bug"
      And I list the yaks in "plain" format
      Then the output should be:
        """
        Write docs
        """

  Rule: Successful removal confirms what was removed

    Example: Removal shows confirmation
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I remove the yak "Fix the bug"
      Then the output should include "Removed 'Fix the bug'"

  Rule: Removing a parent yak without --recursive fails

    Example: Cannot remove a yak with children
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      And I add the yak "Write tests" under "Fix the bug"
      When I try to remove the yak "Fix the bug"
      Then the command should fail
      And the error should contain "--recursive"

  Rule: Recursive removal deletes the yak and all its descendants

    Example: Remove a parent yak and its children
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      And I add the yak "Write tests" under "Fix the bug"
      And I add the yak "Write docs" under "Fix the bug"
      When I remove the yak "Fix the bug" recursively
      And I list the yaks in "plain" format
      Then the output should be empty

    Example: Remove a deeply nested subtree
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      And I add the yak "Write tests" under "Fix the bug"
      And I add the yak "Unit tests" under "Write tests"
      And I add the yak "Keep this"
      When I remove the yak "Fix the bug" recursively
      And I list the yaks in "plain" format
      Then the output should be:
        """
        Keep this
        """

  Rule: Removing a non-existent yak returns an error

    Example: Yak not found
      Given I have a clean git repository
      When I try to remove the yak "Ghost yak"
      Then the command should fail
      And the error should contain "not found"
