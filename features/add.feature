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
    Individual character validation is covered by unit tests.
    This acceptance test verifies the error surfaces correctly.

    Example: Forbidden character is rejected with a clear error
      Given I have a clean git repository
      When I try to add the yak "foo:bar"
      Then the command should fail
      And the error should contain "Invalid yak name"
