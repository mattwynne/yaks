Feature: Show tags on yaks
  Tags should appear in yx show and yx list output.

  Rule: Tags appear in yx show output

    @fullstack
    Example: Show a yak with tags
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0"
      And I run yx show "my yak"
      Then the output should include "@v1.0"

    @fullstack
    Example: Show a yak with multiple tags
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0" and "needs-review"
      And I run yx show "my yak"
      Then the output should include "@v1.0"
      And the output should include "@needs-review"

    @fullstack
    Example: Show a yak without tags has no @ in output
      Given I have a clean git repository
      And I add the yak "my yak"
      When I run yx show "my yak"
      Then the output should not include "@"

  Rule: Tags appear in yx list output

    @fullstack
    Example: List shows tags inline
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0"
      And I run yx list
      Then the output should include "@v1.0"

  Rule: Tags appear in yx show --format json output

    @fullstack
    Example: JSON includes tags array
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1.0" and "needs-review"
      And I run yx show --format json "my yak"
      Then the output should include "tags"
      And the output should include "v1.0"
      And the output should include "needs-review"
