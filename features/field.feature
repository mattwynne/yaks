Feature: Yak fields
  Custom named fields allow storing arbitrary metadata on a yak.

  Rule: Writing and reading fields
    A field can be written via stdin and read back with --show.

    Example: Write a field and show it
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the "notes" field of "my yak" to "field content"
      And I show the "notes" field of "my yak"
      Then the output should be:
        """
        my yak

        field content
        """

  Rule: Zero-byte stdin is a no-op

    @fullstack
    Example: Piping empty content to field does nothing
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the "notes" field of "my yak" with empty stdin
      Then the command should succeed

  Rule: The name field is set automatically on add
    When a yak is added, a "name" field is created containing
    the yak's display name.

    Example: Adding a yak creates a name field
      Given I have a clean git repository
      And I add the yak "my yak"
      When I show the "name" field of "my yak"
      Then the output should be:
        """
        my yak

        my yak
        """

  Rule: The name field is updated on rename
    When a yak is renamed, the name field is updated to
    match the new name.

    Example: Renaming a yak updates its name field
      Given I have a clean git repository
      And I add the yak "old name"
      When I rename the yak "old name" to "new name"
      And I show the "name" field of "new name"
      Then the output should be:
        """
        new name

        new name
        """

  Rule: Reserved field names are rejected
    Certain field names conflict with internal storage and cannot
    be used as custom field names.

    Example: Writing to a reserved field name fails
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the "context.md" field of "my yak" to "content"
      Then the command should fail
      And the error should contain "Field name 'context.md' is reserved"

    Example: Writing to the name field fails
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the "name" field of "my yak" to "custom name"
      Then the command should fail
      And the error should contain "Field name 'name' is reserved"
