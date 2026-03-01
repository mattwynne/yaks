@fullstack
Feature: yx reset - Rebuild yaks from git tree

  Rebuilds the .yaks directory from the tree stored at HEAD of
  refs/notes/yaks. This validates that the git tree is correct
  by materializing it back to the filesystem.

  See also: docs/adr/0010-state-reconstruction-mechanisms.md

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
      And I add the yak "child" under "parent"
      When I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """

    Example: Reset preserves three-level hierarchy
      Given I add the yak "grandparent"
      And I add the yak "parent" under "grandparent"
      And I add the yak "child" under "parent"
      When I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] grandparent
          - [todo] parent
            - [todo] child
        """

  Rule: Reset rebuilds slug-based directories with name and id

    Old-style yak directories used raw names or IDs as directory names.
    Reset rebuilds the disk projection using slugs for directory names,
    with proper `.name` and `.id` files inside.

    Example: Reset creates slug-based directory with name and id files
      Given a yak "my old yak" created with the v2 schema
      When I reset the yaks
      Then the yak "my-old-yak" should have a ".name" file containing "my old yak"
      And the yak "my-old-yak" should have an ".id" file

  Rule: Reset handles duplicate entries in corrupted git trees

    When a corrupted git tree contains two entries that resolve
    to the same yak name, reset should deduplicate and succeed.

    Example: Reset deduplicates entries with the same yak name
      Given a corrupted git tree with duplicate entries for "config"
      When I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] config
        """

  Rule: Reset only affects yak entries

    Example: Non-yak files in the yak directory are preserved
      Given I add the yak "my yak"
      And a file "notes.txt" exists in the yak directory
      When I reset the yaks
      Then the file "notes.txt" should still exist in the yak directory

  Rule: Reset preserves state changes on nested yaks

    Example: Reset preserves state changes on nested yaks
      Given I add the yak "parent"
      And I add the yak "child" under "parent"
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
      And I add the yak "child" under "parent"
      When I rename the yak "child" to "renamed child"
      And I reset the yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] renamed child
        """

  Rule: Reset from disk replays yaks through the Application layer

    When --git-from-disk is used, the git event history is wiped
    and rebuilt by replaying each yak through AddYak. This produces
    clean individual event commits.

    Example: Reset from disk preserves yak data
      Given I add the yak "alpha"
      And I add the yak "beta" under "alpha"
      When I set the state of "beta" to "wip"
      And I reset the yaks from disk to git
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] alpha
          - [wip] beta
        """

    Example: Reset from disk preserves context and custom fields
      When I add the yak "my yak" with context "important notes"
      And I set the "plan" field of "my yak" to "step one"
      And I reset the yaks from disk to git
      And I show the context of "my yak"
      Then the output should include "important notes"
      When I show the "plan" field of "my yak"
      Then the output should include "step one"

    Example: Reset from disk produces clean event log
      Given I add the yak "one"
      And I add the yak "two"
      When I reset the yaks from disk to git
      And I run yx log
      Then the output should include "Added"
      And the output should not include "Snapshot"

    Example: Reset from disk preserves author in event log
      Given I add the yak "my yak"
      When I reset the yaks from disk to git
      And I run yx log
      Then the output should include "<"

    Example: Reset from disk uses current user for legacy yaks without metadata
      Given a yak "legacy yak" created with the v2 schema
      When I reset the yaks from disk to git
      And I run yx log
      Then the output should not include "unknown"

  Rule: reset --git-from-disk requires confirmation

    Example: Declining confirmation aborts reset
      Given I add the yak "my yak"
      When I try to reset from disk
      Then the output should include "Aborted"
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] my yak
        """

    Example: --force skips confirmation
      Given I add the yak "my yak"
      When I reset from disk with --force
      Then the output should include "Reset from disk"
