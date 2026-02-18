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

  Rule: Reset migrates old-style yaks to include name and id

    Old-style yak directories (created before the identity refactor) lack
    `name` and `id` files. Reset runs the v2→v3 migration which adds them.

    Example: Reset adds name and id files to old-style yaks
      Given a yak "my old yak" created with the v2 schema
      When I reset the yaks
      Then the yak "my old yak" should have a "name" file containing "my old yak"
      And the yak "my old yak" should have an "id" file

  Rule: Reset only affects yak entries

    Example: Non-yak files in the yak directory are preserved
      Given I add the yak "my yak"
      And a file "notes.txt" exists in the yak directory
      When I reset the yaks
      Then the file "notes.txt" should still exist in the yak directory

  Rule: Reset can rebuild git tree from disk

    Example: Round-trip from disk to git and back preserves yaks
      Given I add the yak "alpha"
      And I add the yak "beta" blocking "alpha"
      When I set the state of "beta" to "wip"
      And I reset the yaks from disk to git
      And I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] alpha
          - [wip] beta
        """

    Example: Reset preserves state changes on nested yaks
      Given I add the yak "parent"
      And I add the yak "child" blocking "parent"
      And I mark the yak "child" as done
      When I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] parent
          - [done] child
        """

    Example: Reset preserves renames on nested yaks
      Given I add the yak "parent"
      And I add the yak "child" blocking "parent"
      When I rename the yak "child" to "renamed child"
      And I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] renamed child
        """
