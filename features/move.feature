Feature: Move and rename yaks
  Renames yaks and reorganizes them in the hierarchy.
  Alias: yx mv

  Rule: A yak can be renamed
    A simple rename changes the name while keeping everything else.

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
