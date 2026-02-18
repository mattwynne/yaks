Feature: Add yaks
  Create new work items to track. Names are free-form: letters, numbers,
  spaces, slashes, and special characters are all allowed. Use --blocks
  to nest under a parent.

  Rule: Yaks can be created by name

    Example: Adding a simple yak
      Given I have a clean git repository
      When I add the yak "Fix the bug"
      And there should be 1 yak

  Rule: Multi-word names work without quotes
    The CLI joins trailing arguments into a single yak name,
    so users can type `yx add this is a test` without quotes.

    @fullstack
    Example: Separate arguments are joined into one name
      Given I have a clean git repository
      When I run yx add this is a test
      And I list the yaks in "markdown" format
      Then the output should include "this is a test"

  Rule: The assigned ID is echoed on success
    So users can capture it (e.g., ID=$(yx add "my task"))

    Example: Adding a yak prints its ID
      Given I have a clean git repository
      When I add the yak "Make the tea"
      Then the output should include "make-the-tea-"

  Rule: Context can be piped via stdin
    When adding a yak, piped stdin is captured as context.

    @fullstack
    Example: Piped content becomes the yak's context
      Given I have a clean git repository
      When I add the yak "my-yak" with context "# My context" from stdin
      And I show the context of "my-yak"
      Then the output should include "# My context"

  Rule: Forward slash is allowed in names
    Names can contain `/` (e.g. "fix CI/CD pipeline") because storage
    uses slugified directory names.

    Example: Forward slash in name is allowed
      Given I have a clean git repository
      When I add the yak "fix CI/CD pipeline"
      And I list the yaks
      Then the output should include "fix CI/CD pipeline"

  Rule: --blocks creates a child under a parent
    The --blocks flag nests the new yak under the specified parent.
    The parent must already exist and be unambiguous.

    Example: Adding a child under a parent
      Given I have a clean git repository
      And I add the yak "parent"
      When I add the yak "child" blocking "parent"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """

    Example: Nonexistent parent is rejected
      Given I have a clean git repository
      When I try to add the yak "child" blocking "nonexistent"
      Then the command should fail
      And the error should contain "not found"

    Example: Ambiguous parent is rejected
      Given I have a clean git repository
      And I add the yak "Fix the build"
      And I add the yak "Fix the tests"
      When I try to add the yak "child" blocking "Fix"
      Then the command should fail
      And the error should contain "ambiguous"
