Feature: Yak identity
  Each yak has a stable ID (immutable), a human-readable name
  (shown in listings), and a slug (used for directory names on disk).

  Rule: Each yak gets a unique, stable ID
    The ID is generated when a yak is created and never changes,
    even across renames. It's based on the name but includes a
    random suffix for uniqueness.

    @fullstack
    Example: A yak has a readable ID based on its name
      Given I have a clean git repository
      When I add the yak "Fix the bug"
      And I show the ".id" field of "Fix the bug"
      Then the output should include "fix-the-bug-"

    @fullstack
    Example: ID persists across a rename
      Given I have a clean git repository
      And I add the yak "old name"
      When I rename the yak "old name" to "new name"
      And I show the ".id" field of "new name"
      Then the output should include "old-name-"

    Example: Child yak ID is based on its own name
      Given I have a clean git repository
      And I add the yak "project"
      And I add the yak "fix the build" under "project"
      When I show the ".id" field of "fix the build"
      Then the output should include "fix-the-build-"

  Rule: Directory names are human-readable slugs
    On disk, yak directories use a slugified version of the name
    (lowercase, hyphenated, no random suffix) rather than the ID.

    @fullstack
    Example: Directory is named by slug, not ID
      Given I have a clean git repository
      When I add the yak "My Cool Yak"
      Then the yak directory should be named "my-cool-yak"

    @fullstack
    Example: Special characters are stripped from the slug
      Given I have a clean git repository
      When I add the yak "clean up tests & docs!"
      Then the yak directory should be named "clean-up-tests-docs"

    @fullstack
    Example: Nested yak is stored under parent's slug directory
      Given I have a clean git repository
      And I add the yak "My Project"
      And I add the yak "Fix the Build" under "My Project"
      Then the yak directory should be named "my-project/fix-the-build"

  Rule: Listing shows names, not IDs or slugs

    Example: Listing displays the original name
      Given I have a clean git repository
      And I add the yak "Fix the Bug"
      When I list the yaks in "plain" format
      Then the output should be:
        """
        Fix the Bug
        """

  Rule: Reject add when slug collides with sibling

    Example: Adding a yak with colliding slug at root is rejected
      Given I have a clean git repository
      And I add the yak "Make the tea"
      When I try to add the yak "make-the-tea"
      Then the command should fail
      And the error should contain "already exists with the same slug"

    Example: Colliding slug under same parent is rejected
      Given I have a clean git repository
      And I add the yak "Backend fixes"
      And I add the yak "Fix the bug" under "Backend fixes"
      When I try to add the yak "fix-the-bug" under "Backend fixes"
      Then the command should fail
      And the error should contain "already exists under"

    Example: Same slug under different parent succeeds
      Given I have a clean git repository
      And I add the yak "Make the tea"
      And I add the yak "Backend fixes"
      When I add the yak "Make the tea" under "Backend fixes"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] Backend fixes
          - [todo] Make the tea
        - [todo] Make the tea
        """

  Rule: Reject rename when slug collides with sibling
    Note: rename/move slug collision is tested at the domain level
    (unit tests in yak_map.rs). The full-stack flow has a fuzzy match
    layer that reinterprets some renames as parent-moves, so the
    acceptance test uses add-based collisions instead.

    Example: Renaming to collide with sibling is rejected
      Given I have a clean git repository
      And I add the yak "Make the tea"
      And I add the yak "Fix the bug"
      When I try to rename the yak "Fix the bug" to "make-the-tea"
      Then the command should fail
      And the error should contain "already exists"
