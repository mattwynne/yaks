Feature: Add yaks
  Create new work items to track.

  Rule: Yaks can be created by name

    Example: Adding a simple yak
      Given I have a clean git repository
      When I add the yak "Fix the bug"
      And there should be 1 yak

  Rule: Forward slash creates parent-child hierarchy

    Example: Nested yak names use forward slash
      Given I have a clean git repository
      When I add the yak "foo/bar"
      And there should be 2 yaks

  Rule: Invalid characters are rejected
    Names cannot contain: \ : * ? | < > "

    Example: Backslash is rejected
      Given I have a clean git repository
      When I try to add the yak "foo\bar"
      Then the command should fail
      And the error should contain "Invalid yak name"

    Example: Colon is rejected
      Given I have a clean git repository
      When I try to add the yak "foo:bar"
      Then the command should fail
      And the error should contain "Invalid yak name"

    Example: Asterisk is rejected
      Given I have a clean git repository
      When I try to add the yak "foo*bar"
      Then the command should fail
      And the error should contain "Invalid yak name"

    Example: Question mark is rejected
      Given I have a clean git repository
      When I try to add the yak "foo?bar"
      Then the command should fail
      And the error should contain "Invalid yak name"

    Example: Pipe is rejected
      Given I have a clean git repository
      When I try to add the yak "foo|bar"
      Then the command should fail
      And the error should contain "Invalid yak name"

    Example: Less than is rejected
      Given I have a clean git repository
      When I try to add the yak "foo<bar"
      Then the command should fail
      And the error should contain "Invalid yak name"

    Example: Greater than is rejected
      Given I have a clean git repository
      When I try to add the yak "foo>bar"
      Then the command should fail
      And the error should contain "Invalid yak name"
