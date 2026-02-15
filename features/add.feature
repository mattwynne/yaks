Feature: Add yaks
  Create new work items to track. Valid names contain letters, numbers,
  spaces, hyphens, underscores, and forward slash (for nesting).

  Rule: Yaks can be created by name

    Example: Adding a simple yak
      Given I have a clean git repository
      When I add the yak "Fix the bug"
      And there should be 1 yak

  Rule: Forward slash creates parent-child hierarchy

    Example: Nested yak names use forward slash
      Given I have a clean git repository
      When I add the yak "foo/bar"
      And there should be 2 yaks

  Rule: Multi-word names work without quotes
    The CLI joins trailing arguments into a single yak name,
    so users can type `yx add this is a test` without quotes.

    @fullstack
    Example: Separate arguments are joined into one name
      Given I have a clean git repository
      When I run yx add this is a test
      And I list the yaks in "markdown" format
      Then the output should include "this is a test"

  Rule: Context can be piped via stdin
    When adding a yak, piped stdin is captured as context.

    @fullstack
    Example: Piped content becomes the yak's context
      Given I have a clean git repository
      When I add the yak "my-yak" with context "# My context" from stdin
      And I show the context of "my-yak"
      Then the output should include "# My context"

  Rule: Invalid characters are rejected
    Names cannot contain: \ : * ? | < > "
    Individual character validation is covered by unit tests.
    This acceptance test verifies the error surfaces correctly.

    Example: Forbidden character is rejected with a clear error
      Given I have a clean git repository
      When I try to add the yak "foo:bar"
      Then the command should fail
      And the error should contain "Invalid yak name"
