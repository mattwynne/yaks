Feature: Move and rename yaks
  Renames yaks and reorganizes them in the hierarchy.
  Alias: yx mv. All data (context, state, children) is preserved
  when moving. The destination parent must already exist.

  Rule: A yak can be renamed
    A simple rename changes the name while preserving context, state,
    and children.

    Example: Rename a yak
      Given I have a clean git repository
      And I add the yak "old name"
      When I move the yak "old name" to "new name"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] new name
        """

  Rule: A yak can be nested under a parent using slash syntax
    When the destination contains a forward slash, the yak becomes
    a child of the specified parent.

    Example: Move a flat yak into a nested position
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "standalone"
      When I move the yak "standalone" to "parent/child"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """

  Rule: Parent-only destination preserves the source name
    When the destination is the name of an existing yak (with no
    slash), the source yak becomes a child, keeping its original name.

    Example: Move a yak under an existing parent
      Given I have a clean git repository
      And I add the yak "child-yak"
      And I add the yak "parent"
      When I move the yak "child-yak" to "parent"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child-yak
        """

  Rule: Renaming emits a Renamed event
    When the destination is a new name at the same hierarchy level,
    the operation is a rename, not a move.

    Example: Bare rename emits Renamed event in the log
      Given I have a clean git repository
      And I add the yak "old name"
      When I move the yak "old name" to "new name"
      And I run yx log
      Then the output should include "Renamed"
      And the output should not include "Moved"

  Rule: Hierarchy change emits a Moved event
    When the destination has a different parent than the source,
    the operation is a move, not a rename.

    Example: Moving to a new parent emits Moved event
      Given I have a clean git repository
      And I add the yak "child"
      And I add the yak "parent"
      When I move the yak "child" to "parent/child"
      And I run yx log
      Then the output should include "Moved"
      And the output should not include "Renamed"
