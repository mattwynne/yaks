Feature: Yak log
  Every yak command is recorded in a log so you can see
  what happened and when.

  Rule: Log records yak lifecycle events

    Example: Adding a yak produces an event in the log
      Given I have a clean git repository
      And I add the yak "test yak"
      When I run yx log
      Then it should succeed
      And the output should include "test yak"

  Rule: Log displays events in chronological order

    Example: Events appear oldest-first
      Given I have a clean git repository
      And I add the yak "first yak"
      And I add the yak "second yak"
      When I run yx log
      Then it should succeed
      And line 1 of the output should include "first yak"
      And line 5 of the output should include "second yak"

  Rule: Log entries use narrative format

    Each entry reads as a natural English sentence with the
    author as subject, followed by metadata on subsequent lines,
    separated by horizontal rules.

    @fullstack
    Example: Added event shows narrative with author and yak name
      Given I have a clean git repository
      And I add the yak "Fix the Bug"
      When I run yx log
      Then it should succeed
      And line 1 of the output should include "added Fix the Bug"
      And line 3 of the output should include "event:"
      And line 3 of the output should include "sha:"
      And the output should include "────"

  Rule: State changes use human-friendly verbs

    Example: Starting a yak says "started"
      Given I have a clean git repository
      And I add the yak "Fix the Bug"
      When I set the state of "Fix the Bug" to "wip"
      And I run yx log
      Then the output should include "started Fix the Bug"

    Example: Finishing a yak says "finished"
      Given I have a clean git repository
      And I add the yak "Fix the Bug"
      When I set the state of "Fix the Bug" to "done"
      And I run yx log
      Then the output should include "finished Fix the Bug"

    Example: Resetting a yak says "reset to todo"
      Given I have a clean git repository
      And I add the yak "Fix the Bug"
      And I set the state of "Fix the Bug" to "wip"
      When I set the state of "Fix the Bug" to "todo"
      And I run yx log
      Then the output should include "reset Fix the Bug to todo"
