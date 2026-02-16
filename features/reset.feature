@fullstack
Feature: yx reset - Rebuild yaks from git tree

  Rebuilds the .yaks directory from the tree stored at HEAD of
  refs/notes/yaks. This validates that the git tree is correct
  by materializing it back to the filesystem.

  Background:
    Given I have a clean git repository

  Rule: Reset rebuilds yaks from the git event store tree

    Example: Reset after adding a yak and changing state
      Given I add the yak "my yak"
      When I set the state of "my yak" to "wip"
      And I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] my yak
        """

    Example: Reset preserves parent-child hierarchy
      Given I add the yak "parent"
      And I add the yak "child" blocking "parent"
      When I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """
