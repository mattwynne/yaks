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
      And line 5 of the output should include "first yak"
      And line 11 of the output should include "second yak"

  Rule: Events reference yaks by ID

    Example: Added event contains the yak ID
      Given I have a clean git repository
      And I add the yak "Fix the Bug"
      When I run yx log
      Then the output should include "fix-the-bug-"

  Rule: Log entries use git-log style format

    @fullstack
    Example: Log entries show event ID, author, date, and message
      Given I have a clean git repository
      And I add the yak "test yak"
      When I run yx log
      Then it should succeed
      And line 1 of the output should include "event "
      And line 2 of the output should include "Author:"
      And line 3 of the output should include "Date:"
      And line 5 of the output should include "Added"
