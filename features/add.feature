Feature: Add yaks
  Create new work items to track.

  Rule: Yaks can be created by name

    Example: Adding a simple yak
      Given I have a clean git repository
      When I add the yak "Fix the bug"
      And there should be 1 yak

  Rule: Invalid characters are rejected
    Names cannot contain: \ : * ? | < > "

    Example: Backslash is rejected
      Given I have a clean git repository
      When I try to add the yak "foo\bar"
      Then the command should fail
      And the error should contain "Invalid yak name"
