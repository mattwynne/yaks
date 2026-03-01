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
        field content
        """

  Rule: Zero-byte stdin is a no-op

    @fullstack
    Example: Piping empty content to field does nothing
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the "notes" field of "my yak" with empty stdin
      Then the command should succeed

  Rule: Bare field defaults to showing field

    @fullstack
    Example: bare field shows field content
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the "notes" field of "my yak" to "my notes"
      And I run yx field "my yak" notes
      Then the output should be:
        """
        my notes
        """

  Rule: --edit launches editor for field

    @fullstack
    Example: --edit opens editor with current field value
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the "notes" field of "my yak" to "original"
      And I edit the "notes" field of "my yak" with editor that appends " - edited"
      And I show the "notes" field of "my yak"
      Then the output should include "original - edited"

  Rule: piped stdin + --edit opens editor pre-populated

    @fullstack
    Example: piped stdin becomes initial editor content for field
      Given I have a clean git repository
      And I add the yak "my yak"
      When I pipe "seed notes" and edit the "notes" field of "my yak" with editor that appends " - edited"
      And I show the "notes" field of "my yak"
      Then the output should include "seed notes - edited"

  Rule: The .name field is set automatically on add
    When a yak is added, a ".name" field is created containing
    the yak's display name. Reserved fields use dot-prefix to
    avoid collision with user-defined fields.

    Example: Adding a yak creates a .name field
      Given I have a clean git repository
      And I add the yak "my yak"
      When I show the ".name" field of "my yak"
      Then the output should be:
        """
        my yak
        """

  Rule: The .name field is updated on rename
    When a yak is renamed, the .name field is updated to
    match the new name.

    Example: Renaming a yak updates its .name field
      Given I have a clean git repository
      And I add the yak "old name"
      When I rename the yak "old name" to "new name"
      And I show the ".name" field of "new name"
      Then the output should be:
        """
        new name
        """

  Rule: Reserved field names are rejected
    Dot-prefixed reserved field names cannot be used as custom
    field names. They are used internally by yx.

    Example: Writing to a reserved field name fails
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the ".context.md" field of "my yak" to "content"
      Then the command should fail
      And the error should contain "Field name '.context.md' is reserved"

    Example: Writing to the .name field fails
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the ".name" field of "my yak" to "custom name"
      Then the command should fail
      And the error should contain "Field name '.name' is reserved"

  Rule: --show and piped stdin are mutually exclusive

    @fullstack
    Example: --show with piped input is rejected
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to show "notes" field of "my yak" with piped input "some content"
      Then the command should fail
      And the error should contain "Cannot use --show when piping input"

  Rule: --show and --edit are mutually exclusive

    @fullstack
    Example: --show with --edit is rejected
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to run yx field "my yak" notes --show --edit
      Then the command should fail
      And the error should contain "Cannot use both --show and --edit"
