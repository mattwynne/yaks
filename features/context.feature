Feature: Manage yak context
  Adds detailed notes, requirements, or background to yaks.

  Context is stored per-yak and can be set from stdin (pipeline mode)
  or edited interactively ($EDITOR). The --show flag displays the yak
  name followed by its context.

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

  Rule: Show mode displays yak name and context

    Example: Showing a yak with no context shows only the name
      Given I have a clean git repository
      And I add the yak "my yak"
      When I show the context of "my yak"
      Then the output should be:
        """
        my yak
        """
