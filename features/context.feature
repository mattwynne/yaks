Feature: Manage yak context
  Adds detailed notes, requirements, or background to yaks.

  Context is stored per-yak and can be set from stdin (pipeline mode)
  or edited interactively ($EDITOR). The --show flag displays the raw
  context content. Keep yak names short and use context for detailed
  requirements, acceptance criteria, and technical notes.

  Rule: Context can be set from stdin

    Example: Setting context from stdin and showing it
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the context of "my yak" to "# Some context"
      And I show the context of "my yak"
      Then the output should be:
        """
        # Some context
        """

    Example: Setting context from a file redirect
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the context of "my yak" from a file containing "# File context"
      And I show the context of "my yak"
      Then the output should be:
        """
        # File context
        """

  Rule: Stdin input replaces existing context

    Example: Setting context twice replaces the first value
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the context of "my yak" to "old"
      And I set the context of "my yak" to "new"
      And I show the context of "my yak"
      Then the output should be:
        """
        new
        """

  Rule: Zero-byte stdin is a no-op

    @fullstack
    Example: Piping empty content to context does nothing
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to set the context of "my yak" with empty stdin
      Then the command should succeed

  Rule: Bare context defaults to showing context

    The routing logic lives in main.rs, so this only
    applies to the fullstack (binary) test path.

    @fullstack
    Example: bare context shows nothing when no context is set
      Given I have a clean git repository
      And I add the yak "my yak"
      When I run yx context "my yak"
      Then the output should be:
        """
        """

  Rule: --edit launches editor for context

    @fullstack
    Example: --edit opens editor with current context
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the context of "my yak" to "original"
      And I edit the context of "my yak" with editor that appends " - edited"
      And I show the context of "my yak"
      Then the output should include "original - edited"

  Rule: piped stdin + --edit opens editor pre-populated

    @fullstack
    Example: piped stdin becomes initial editor content
      Given I have a clean git repository
      And I add the yak "my yak"
      When I pipe "seed content" and edit the context of "my yak" with editor that appends " - edited"
      And I show the context of "my yak"
      Then the output should include "seed content - edited"

  Rule: Show mode displays just the raw context

    Example: Showing a yak with no context produces no output
      Given I have a clean git repository
      And I add the yak "my yak"
      When I show the context of "my yak"
      Then the output should be:
        """
        """

  Rule: --show and piped stdin are mutually exclusive

    @fullstack
    Example: --show with piped input is rejected
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to show context of "my yak" with piped input "some content"
      Then the command should fail
      And the error should contain "Cannot use --show when piping input"

  Rule: --show and --edit are mutually exclusive

    @fullstack
    Example: --show with --edit is rejected
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to run yx context "my yak" --show --edit
      Then the command should fail
      And the error should contain "Cannot use both --show and --edit"
