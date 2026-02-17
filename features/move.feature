Feature: Move yaks in hierarchy
  Moves yaks between positions in the hierarchy using --under and
  --to-root flags. Alias: yx mv. All data (context, state) is
  preserved when moving.

  Rule: --under moves a yak under a parent

    Example: Move a yak under an existing parent
      Given I have a clean git repository
      And I add the yak "child-yak"
      And I add the yak "parent"
      When I move the yak "child-yak" under "parent"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child-yak
        """

  Rule: --to-root moves a yak to root level

    Example: Move a nested yak to root
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" blocking "parent"
      When I move the yak "child" to root
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] child
        - [todo] parent
        """

  Rule: --under resolves parent by fuzzy match

    Example: Fuzzy match parent name
      Given I have a clean git repository
      And I add the yak "standalone"
      And I add the yak "Make the tea"
      When I move the yak "standalone" under "the tea"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] Make the tea
          - [todo] standalone
        """

  Rule: --under and --to-root are mutually exclusive

    Example: Using both flags errors
      Given I have a clean git repository
      And I add the yak "foo"
      And I add the yak "bar"
      When I try to move the yak "foo" under "bar" to root
      Then the command should fail

  Rule: mv requires exactly one of --under or --to-root

    Example: Using neither flag errors
      Given I have a clean git repository
      And I add the yak "foo"
      When I try to move the yak "foo" with no flags
      Then the command should fail

  Rule: Hierarchy change emits a Moved event

    Example: Moving under a parent emits Moved event
      Given I have a clean git repository
      And I add the yak "child"
      And I add the yak "parent"
      When I move the yak "child" under "parent"
      And I run yx log
      Then the output should include "Moved"
      And the output should not include "Renamed"
