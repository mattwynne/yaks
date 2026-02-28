Feature: Yak tags
  Tags allow labeling yaks with short identifiers for categorization.

  Rule: Adding tags to a yak
    Tags are stored in the "tags" field, displayed with @ prefix.

    Example: Add a single tag
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0"
      And I list tags on "my yak"
      Then the output should be:
        """
        @v1.0
        """

    Example: Add multiple tags at once
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0" and "needs-review"
      And I list tags on "my yak"
      Then the output should be:
        """
        @v1.0
        @needs-review
        """

    Example: Leading @ is stripped on input
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "@v1.0"
      And I list tags on "my yak"
      Then the output should be:
        """
        @v1.0
        """

    Example: Adding a duplicate tag is a no-op
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0"
      And I tag "my yak" with "v1.0"
      And I list tags on "my yak"
      Then the output should be:
        """
        @v1.0
        """

  Rule: Removing tags from a yak

    Example: Remove a tag
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0" and "needs-review"
      And I remove the tag "v1.0" from "my yak"
      And I list tags on "my yak"
      Then the output should be:
        """
        @needs-review
        """

    Example: Removing a non-existent tag is a no-op
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0"
      And I remove the tag "no-such-tag" from "my yak"
      And I list tags on "my yak"
      Then the output should be:
        """
        @v1.0
        """

  Rule: Listing tags on a yak with no tags

    Example: List tags when none exist
      Given I have a clean git repository
      And I add the yak "my yak"
      When I list tags on "my yak"
      Then the output should be empty

  Rule: CLI aliases

    @fullstack
    Example: yx tags works as synonym for yx tag
      Given I have a clean git repository
      And I add the yak "my yak"
      When I run yx tag add "my yak" v1.0
      And I run yx tags list "my yak"
      Then the output should be:
        """
        @v1.0
        """

    @fullstack
    Example: yx tag remove works as synonym for yx tag rm
      Given I have a clean git repository
      And I add the yak "my yak"
      When I run yx tag add "my yak" v1.0
      And I run yx tag remove "my yak" v1.0
      And I run yx tag list "my yak"
      Then the output should be empty
