Feature: Find yaks by ID suffix or name
  Commands that take a yak accept either the unique suffix of its ID
  or a unique substring of its name. This avoids needing to type or
  remember long IDs and hierarchical names.

  Rule: A unique ID suffix matches a yak

    Example: Marking a yak done by its unique ID suffix
      Given I have a clean git repository
      And I add the yak "stray click" with id "stray-click-wx09"
      And I mark the yak "wx09" as done
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [done] stray click
        """

  Rule: A unique name substring matches a yak

    Example: Marking a yak done by unique substring
      Given I have a clean git repository
      And I add the yak "ideas"
      And I add the yak "buy a pony" under "ideas"
      And I add the yak "fix the build" under "ideas"
      And I add the yak "fix the fridge" under "ideas"
      And I mark the yak "build" as done
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] ideas
          - [done] fix the build
          - [todo] buy a pony
          - [todo] fix the fridge
        """

  Rule: An ambiguous substring produces a clear error

    Example: Failing with an ambiguous match error
      Given I have a clean git repository
      And I add the yak "ideas"
      And I add the yak "buy a pony" under "ideas"
      And I add the yak "fix the build" under "ideas"
      And I add the yak "fix the fridge" under "ideas"
      When I try to mark the yak "fix" as done
      Then the command should fail
      And the error should contain "ambiguous"

  Rule: A parent name is not ambiguous with its children

    Example: Setting context on a parent that has children
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child1" under "parent"
      When I set the context of "parent" to "test context"
      And I show the context of "parent"
      Then the output should include "test context"

  Rule: Fuzzy matching finds parent yaks too

    Example: A parent yak can be found by fuzzy match
      Given I have a clean git repository
      And I add the yak "project"
      And I add the yak "fix the build" under "project"
      When I set the state of "proj" to "wip"
      And I show the ".state" field of "project"
      Then the output should include "wip"
