Feature: Manage yak context
  Adds detailed notes, requirements, or background to yaks.

  Context is stored per-yak and can be set from stdin (pipeline mode)
  or edited interactively ($EDITOR). The --show flag displays the yak
  name followed by its context. Keep yak names short and use context
  for detailed requirements, acceptance criteria, and technical notes.

  Rule: Context can be set from stdin

    Example: Setting context from stdin and showing it
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the context of "my yak" to "# Some context"
      And I show the context of "my yak"
      Then the output should be:
        """
        my yak

        # Some context
        """

    Example: Setting context from a file redirect
      Given I have a clean git repository
      And I add the yak "my yak"
      When I set the context of "my yak" from a file containing "# File context"
      And I show the context of "my yak"
      Then the output should be:
        """
        my yak

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
        my yak

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
    Example: bare context shows the yak name
      Given I have a clean git repository
      And I add the yak "my yak"
      When I run yx context "my yak"
      Then the output should be:
        """
        my yak
        """

  Rule: Show mode displays yak name and context

    Example: Showing a yak with no context shows only the name
      Given I have a clean git repository
      And I add the yak "my yak"
      When I show the context of "my yak"
      Then the output should be:
        """
        my yak
        """
