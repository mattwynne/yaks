Feature: Add yaks
  Create new work items to track. Names are free-form: letters, numbers,
  spaces, slashes, and special characters are all allowed. Use --under
  (or --below, --in, --into, --blocks) to nest under a parent.

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

  Rule: --under creates a child under a parent
    The --under flag nests the new yak under the specified parent.
    --below, --in, --into, and --blocks are synonyms. The parent must
    already exist and be unambiguous.

    Example: Adding a child under a parent
      Given I have a clean git repository
      And I add the yak "parent"
      When I add the yak "child" under "parent"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """

    @fullstack
    Scenario Outline: Synonyms for --under
      Given I have a clean git repository
      And I add the yak "parent"
      When I run yx add child <flag> parent
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """

      Examples:
        | flag     |
        | --under  |
        | --below  |
        | --in     |
        | --into   |
        | --blocks |

    Example: Adding a child to a done parent sets the parent back to todo
      Given I have a clean git repository
      And I add the yak "parent"
      And I mark the yak "parent" as done
      When I add the yak "child" under "parent"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child
        """

    Example: Adding a child to a done grandparent sets ancestors back to todo
      Given I have a clean git repository
      And I add the yak "grandparent"
      And I add the yak "parent" under "grandparent"
      And I mark the yak "parent" as done
      And I mark the yak "grandparent" as done
      When I add the yak "child" under "parent"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] grandparent
          - [todo] parent
            - [todo] child
        """

    Example: Nonexistent parent is rejected
      Given I have a clean git repository
      When I try to add the yak "child" under "nonexistent"
      Then the command should fail
      And the error should contain "not found"

    Example: Ambiguous parent is rejected
      Given I have a clean git repository
      And I add the yak "Fix the build"
      And I add the yak "Fix the tests"
      When I try to add the yak "child" under "Fix"
      Then the command should fail
      And the error should contain "ambiguous"

  Rule: --state sets the initial state

    Example: Add yak with initial state
      Given I have a clean git repository
      When I add the yak "my yak" with state "wip"
      And I list the yaks in "markdown" format
      Then the output should include "wip"

    @fullstack
    Example: --state flag via CLI
      Given I have a clean git repository
      When I run yx add "my yak" --state wip
      And I list the yaks in "markdown" format
      Then the output should include "wip"

  Rule: --context sets context directly

    Example: Add yak with context
      Given I have a clean git repository
      When I add the yak "my yak" with context "some notes"
      And I show the context of "my yak"
      Then the output should include "some notes"

    @fullstack
    Example: --context flag via CLI
      Given I have a clean git repository
      When I run yx add "my yak" --context "some notes"
      And I run yx context --show "my yak"
      Then the output should include "some notes"

  Rule: --id assigns a specific ID

    Example: Add yak with explicit ID
      Given I have a clean git repository
      When I add the yak "my yak" with id "custom-id-1234"
      Then the output should include "custom-id-1234"

    @fullstack
    Example: --id flag via CLI
      Given I have a clean git repository
      When I run yx add "my yak" --id "custom-id-1234"
      Then the output should include "custom-id-1234"

  Rule: bare add creates yak with no context

    Example: add without --context or --edit sets no context
      Given I have a clean git repository
      And I add the yak "my yak"
      When I show the context of "my yak"
      Then the output should be:
        """
        """

  Rule: --edit launches editor for initial context

    @fullstack
    Example: --edit and --context together is an error
      Given I have a clean git repository
      When I try to run yx add "my yak" --edit --context "notes"
      Then the command should fail

    @fullstack
    Example: --edit opens $EDITOR, saved content becomes context
      Given I have a clean git repository
      When I add the yak "my yak" with editor that writes "edited context"
      And I show the context of "my yak"
      Then the output should include "edited context"

  Rule: --field sets custom fields at creation time

    Example: Add yak with custom field
      Given I have a clean git repository
      When I add the yak "my yak" with field "priority" set to "high"
      And I show the "priority" field of "my yak"
      Then the output should include "high"

    @fullstack
    Example: --field flag via CLI
      Given I have a clean git repository
      When I run yx add "my yak" --field priority=high
      And I run yx field "my yak" priority --show
      Then the output should include "high"

  Rule: Whitespace-only names are rejected

    Example: Whitespace-only name is rejected
      Given I have a clean git repository
      When I try to add the yak "   "
      Then the command should fail
      And the error should contain "cannot be empty"
