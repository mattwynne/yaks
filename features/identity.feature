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
      And I show the "id" field of "Fix the bug"
      Then the output should include "fix-the-bug-"

    @fullstack
    Example: ID persists across a rename
      Given I have a clean git repository
      And I add the yak "old name"
      When I move the yak "old name" to "new name"
      And I show the "id" field of "new name"
      Then the output should include "old-name-"

  Rule: Directory names are human-readable slugs
    On disk, yak directories use a slugified version of the name
    (lowercase, hyphenated, no random suffix) rather than the ID.

    @fullstack
    Example: Directory is named by slug, not ID
      Given I have a clean git repository
      When I add the yak "My Cool Yak"
      Then the yak directory should be named "my-cool-yak"

  Rule: Listing shows names, not IDs or slugs

    Example: Listing displays the original name
      Given I have a clean git repository
      And I add the yak "Fix the Bug"
      When I list the yaks in "plain" format
      Then the output should be:
        """
        Fix the Bug
        """
