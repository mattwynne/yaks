Feature: List yaks
  Displays all yaks with their status and hierarchy.

  The default format is "pretty" (tree-style with Unicode indicators).
  Machine-readable formats "markdown" and "plain" are also available.
  Alias: `yx ls`

  # Default (pretty) format

  Scenario: List a single yak
    Given I have a clean git repository
    And I add the yak "Fix the bug"
    When I list the yaks
    Then the output should be:
      """
        ○ Fix the bug
      """

  Scenario: Show nothing when no yaks exist
    Given I have a clean git repository
    When I list the yaks
    Then the output should be empty

  # Markdown format

  Scenario: List yaks in markdown format
    Given I have a clean git repository
    And I add the yak "Fix the bug"
    When I list the yaks in "markdown" format
    Then the output should be:
      """
      - [todo] Fix the bug
      """

  Scenario: Show message when no yaks exist in markdown format
    Given I have a clean git repository
    When I list the yaks in "markdown" format
    Then the output should be:
      """
      You have no yaks. Are you done?
      """

  Scenario: Support "md" as alias for markdown format
    Given I have a clean git repository
    And I add the yak "Fix the bug"
    When I list the yaks in "md" format
    Then the output should be:
      """
      - [todo] Fix the bug
      """

  Scenario: Sort sibling yaks with done first, then alphabetically
    Given I have a clean git repository
    And I add the yak "zebra"
    And I add the yak "mango"
    And I add the yak "apple"
    And I mark the yak "apple" as done
    When I list the yaks in "markdown" format
    Then the output should be:
      """
      - [done] apple
      - [todo] mango
      - [todo] zebra
      """

  Scenario: Display nested yaks with indentation
    Given I have a clean git repository
    And I add the yak "first task"
    And I add the yak "first task/second task"
    When I list the yaks in "markdown" format
    Then the output should be:
      """
      - [todo] first task
        - [todo] second task
      """

  Scenario: Keep hierarchy when child is done
    Given I have a clean git repository
    And I add the yak "parent a"
    And I add the yak "parent a/child 1"
    And I add the yak "parent a/child 2"
    And I mark the yak "parent a/child 1" as done
    And I add the yak "parent b"
    When I list the yaks in "markdown" format
    Then the output should be:
      """
      - [wip] parent a
        - [done] child 1
        - [todo] child 2
      - [todo] parent b
      """

  # Plain format (for scripting)

  Scenario: List yaks in plain format
    Given I have a clean git repository
    And I add the yak "Fix the bug"
    When I list the yaks in "plain" format
    Then the output should be:
      """
      Fix the bug
      """

  Scenario: Show nested yaks with full paths in plain format
    Given I have a clean git repository
    And I add the yak "parent task"
    And I add the yak "parent task/child task"
    When I list the yaks in "plain" format
    Then the output should be:
      """
      parent task
      parent task/child task
      """

  Scenario: Support "raw" as alias for plain format
    Given I have a clean git repository
    And I add the yak "Fix the bug"
    When I list the yaks in "raw" format
    Then the output should be:
      """
      Fix the bug
      """

  Scenario: Output nothing in plain format when no yaks exist
    Given I have a clean git repository
    When I list the yaks in "plain" format
    Then the output should be empty

  # Filtering with --only

  Scenario: Filter to show only incomplete yaks
    Given I have a clean git repository
    And I add the yak "incomplete task"
    And I add the yak "done task"
    And I mark the yak "done task" as done
    When I list the yaks in "plain" format filtering by "not-done"
    Then the output should be:
      """
      incomplete task
      """

  Scenario: Filter to show only completed yaks
    Given I have a clean git repository
    And I add the yak "incomplete task"
    And I add the yak "done task"
    And I mark the yak "done task" as done
    When I list the yaks in "plain" format filtering by "done"
    Then the output should be:
      """
      done task
      """

  Scenario: Show all yaks when no filter is specified
    Given I have a clean git repository
    And I add the yak "done task"
    And I add the yak "incomplete task"
    And I mark the yak "done task" as done
    When I list the yaks in "plain" format
    Then the output should be:
      """
      done task
      incomplete task
      """

  Scenario: Include parent when filtering nested yaks by not-done status
    Given I have a clean git repository
    And I add the yak "parent"
    And I add the yak "parent/done child"
    And I add the yak "parent/incomplete child"
    And I mark the yak "parent/done child" as done
    When I list the yaks in "markdown" format filtering by "not-done"
    Then the output should be:
      """
      - [wip] parent
        - [todo] incomplete child
      """

