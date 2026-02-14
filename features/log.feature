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
      And line 2 of the output should include "second yak"
