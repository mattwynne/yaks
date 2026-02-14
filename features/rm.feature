Feature: Remove yaks
  Delete yaks that are no longer needed.

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

  Rule: Successful removal produces no output

    Example: Silent success
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I remove the yak "Fix the bug"
      Then the output should be empty

  Rule: Removing a non-existent yak returns an error

    Example: Yak not found
      Given I have a clean git repository
      When I try to remove the yak "Ghost yak"
      Then the command should fail
      And the error should contain "not found"
