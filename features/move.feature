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

  Rule: --below, --in, --into, and --blocks are synonyms for --under

    @fullstack
    Scenario Outline: Move a yak using a synonym for --under
      Given I have a clean git repository
      And I add the yak "child-yak"
      And I add the yak "parent"
      When I run yx mv child-yak <flag> parent
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] parent
          - [todo] child-yak
        """

      Examples:
        | flag     |
        | --under  |
        | --below  |
        | --in     |
        | --into   |
        | --blocks |

  Rule: --to-root moves a yak to root level

    Example: Move a nested yak to root
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
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

  Rule: Moving a yak with children moves the whole subtree

    Example: Move a subtree under another yak
      Given I have a clean git repository
      And I add the yak "epic"
      And I add the yak "story" under "epic"
      And I add the yak "task" under "story"
      And I add the yak "backlog"
      When I move the yak "epic" under "backlog"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] backlog
          - [todo] epic
            - [todo] story
              - [todo] task
        """

    Example: Move a subtree to root
      Given I have a clean git repository
      And I add the yak "Make the tea"
      And I add the yak "Boil the kettle" under "Make the tea"
      And I add the yak "Fill the kettle" under "Boil the kettle"
      When I move the yak "Boil the kettle" to root
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] Boil the kettle
          - [todo] Fill the kettle
        - [todo] Make the tea
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
      Then the output should include "moved"

  Rule: Moving a yak under itself or its own descendant is rejected

    Example: Cannot move a yak under itself
      Given I have a clean git repository
      And I add the yak "my yak"
      When I try to move the yak "my yak" under "my yak"
      Then the command should fail
      And the error should contain "under itself"

    Example: Cannot move a yak under its own child
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      When I try to move the yak "parent" under "child"
      Then the command should fail
      And the error should contain "descendant"

  Rule: Moving yaks keeps the blocker graph acyclic

    Example: Moving a blocker under the yak it blocks makes the blocker implicit
      Given I have a clean git repository
      And I add the yak "a"
      And I add the yak "b"
      And I add blocker "b" to "a"
      When I move the yak "b" under "a"
      And I show the log
      Then the output should include "removed blocker"
      And the output should include "moved"

    Example: Moving a subtree makes descendant blockers implicit
      Given I have a clean git repository
      And I add the yak "a"
      And I add the yak "b"
      And I add the yak "c" under "b"
      And I add blocker "c" to "a"
      When I move the yak "b" under "a"
      And I list the yaks as json
      Then the JSON yak "a" should not have blockers
      When I show the log
      Then the output should include "removed blocker"
      And the output should include "moved"

    Example: Cannot move a blocked yak under its blocker
      Given I have a clean git repository
      And I add the yak "a"
      And I add the yak "b"
      And I add blocker "b" to "a"
      When I try to move the yak "a" under "b"
      Then the command should fail
      And the error should contain "would create blocker cycle"

    Example: Cannot move an ancestor under an explicit blocker when that would create a cycle
      Given I have a clean git repository
      And I add the yak "project"
      And I add the yak "a" under "project"
      And I add the yak "b"
      And I add blocker "b" to "a"
      When I try to move the yak "project" under "b"
      Then the command should fail
      And the error should contain "would create blocker cycle"
