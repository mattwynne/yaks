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

  Rule: Empty piped stdin is an error

    Example: Piping empty content to field fails
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the "notes" field of "my yak" with empty stdin
      Then the command should fail
      And the error should contain "no content received on stdin"

  Rule: Reserved field names are rejected
    Certain field names conflict with internal storage and cannot
    be used as custom field names.

    Example: Writing to a reserved field name fails
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the "context.md" field of "my yak" to "content"
      Then the command should fail
      And the error should contain "Field name 'context.md' is reserved"
