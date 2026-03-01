Feature: yx rename command
  Changes a yak's display name without affecting its position
  in the hierarchy. The yak's ID stays the same, the slug
  (directory name) updates to match the new name.

  Rule: A yak can be renamed
    Example: Rename a yak
      Given I have a clean git repository
      And I add the yak "Make the tea"
      When I rename the yak "Make the tea" to "Brew the tea"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] Brew the tea
        """

    Example: Rename preserves the yak ID
      Given I have a clean git repository
      And I add the yak "old name"
      When I rename the yak "old name" to "new name"
      And I show the ".id" field of "new name"
      Then the output should include "old-name-"

  Rule: Renaming a child does not move it
    Example: Rename a nested yak
      Given I have a clean git repository
      And I add the yak "project"
      And I add the yak "fix the build" under "project"
      When I rename the yak "fix the build" to "repair the build"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] project
          - [todo] repair the build
        """

  Rule: Rename rejects names that collide with a sibling's slug
    Example: Rename colliding with sibling is rejected
      Given I have a clean git repository
      And I add the yak "Make the tea"
      And I add the yak "Fix the bug"
      When I try to rename the yak "Fix the bug" to "make-the-tea"
      Then the command should fail
      And the error should contain "already exists"

  Rule: Rename emits a FieldUpdated event
    Example: Rename emits FieldUpdated event in the log
      Given I have a clean git repository
      And I add the yak "old name"
      When I rename the yak "old name" to "new name"
      And I run yx log
      Then the output should include "FieldUpdated"

  Rule: Whitespace-only names are rejected

    Example: Cannot rename to whitespace-only name
      Given I have a clean git repository
      And I add the yak "real yak"
      When I try to rename the yak "real yak" to "   "
      Then the command should fail
      And the error should contain "cannot be empty"
