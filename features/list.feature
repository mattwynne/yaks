Feature: List yaks
  Displays all yaks with their status and hierarchy.

  Rule: Multiple output formats are available
    The default is "pretty" (tree-style with Unicode indicators).
    "markdown" shows state labels. "plain" shows leaf names for root yaks
    and slash-separated paths for nested yaks (for scripting).
    Aliases: "md" for markdown, "raw" for plain.

    Example: Pretty format is the default
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I list the yaks
      Then the output should be:
        """
          ○ Fix the bug
        """

    Example: Markdown format shows state in brackets
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] Fix the bug
        """

    Example: "md" is an alias for markdown
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I list the yaks in "md" format
      Then the output should be:
        """
        - [todo] Fix the bug
        """

    Example: Plain format shows just the name
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I list the yaks in "plain" format
      Then the output should be:
        """
        Fix the bug
        """

    Example: "raw" is an alias for plain
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I list the yaks in "raw" format
      Then the output should be:
        """
        Fix the bug
        """

  Rule: Done yaks sort before undone yaks, then alphabetically

    Example: Sort sibling yaks with done first, then alphabetically
      Given I have a clean git repository
      And I add the yak "zebra"
      And I add the yak "mango"
      And I add the yak "apple"
      And I mark the yak "zebra" as done
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [done] zebra
        - [todo] apple
        - [todo] mango
        """

  Rule: Nested yaks are displayed as a hierarchy

    Example: Pretty format shows tree connectors for nested yaks
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child a" under "parent"
      And I add the yak "child b" under "parent"
      When I list the yaks
      Then the output should be:
        """
        ○ parent
         ├─ ○ child a
         ╰─ ○ child b
        """

    Example: Pretty format shows tree connectors for deep nesting
      Given I have a clean git repository
      And I add the yak "root"
      And I add the yak "branch" under "root"
      And I add the yak "leaf" under "branch"
      And I add the yak "sibling" under "root"
      When I list the yaks
      Then the output should be:
        """
        ○ root
         ├─ ○ branch
         │  ╰─ ○ leaf
         ╰─ ○ sibling
        """

    Example: Nested yaks are indented under their parent
      Given I have a clean git repository
      And I add the yak "first task"
      And I add the yak "second task" under "first task"
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] first task
          - [todo] second task
        """

    Example: Done children stay under their parent
      Given I have a clean git repository
      And I add the yak "parent a"
      And I add the yak "child 1" under "parent a"
      And I add the yak "child 2" under "parent a"
      And I mark the yak "child 1" as done
      And I add the yak "parent b"
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] parent a
          - [done] child 1
          - [todo] child 2
        - [todo] parent b
        """

    Example: Plain format shows full paths for nested yaks
      Given I have a clean git repository
      And I add the yak "parent task"
      And I add the yak "child task" under "parent task"
      When I list the yaks in "plain" format
      Then the output should be:
        """
        parent task
        parent task/child task
        """

  Rule: Yaks can be filtered by completion status

    Example: Show only incomplete yaks
      Given I have a clean git repository
      And I add the yak "incomplete task"
      And I add the yak "done task"
      And I mark the yak "done task" as done
      When I list the yaks in "plain" format filtering by "not-done"
      Then the output should be:
        """
        incomplete task
        """

    Example: Show only completed yaks
      Given I have a clean git repository
      And I add the yak "incomplete task"
      And I add the yak "done task"
      And I mark the yak "done task" as done
      When I list the yaks in "plain" format filtering by "done"
      Then the output should be:
        """
        done task
        """

    Example: Show all yaks when no filter is specified
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

    Example: Parents are included when filtering nested yaks
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "done child" under "parent"
      And I add the yak "incomplete child" under "parent"
      And I mark the yak "done child" as done
      When I list the yaks in "markdown" format filtering by "not-done"
      Then the output should be:
        """
        - [wip] parent
          - [todo] incomplete child
        """

  Rule: Empty list shows appropriate feedback per format

    Example: Pretty format shows nothing when empty
      Given I have a clean git repository
      When I list the yaks
      Then the output should be empty

    Example: Markdown format shows a friendly message when empty
      Given I have a clean git repository
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        You have no yaks. Are you done?
        """

    Example: Plain format shows nothing when empty
      Given I have a clean git repository
      When I list the yaks in "plain" format
      Then the output should be empty

  Rule: JSON output for agent/script consumption

    The --format json option outputs the full yak tree as a JSON array.
    Each yak includes id, name, state, context, parent_id, tags,
    fields, and children (nested recursively).

    Example: Empty list returns empty JSON array
      Given I have a clean git repository
      When I list the yaks as json
      Then the output should be:
        """
        []
        """

    Example: Single yak outputs JSON with correct fields
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I set the context of "Fix the bug" to "It crashes on startup"
      And I list the yaks as json
      Then the output should include "Fix the bug"
      And the output should include "todo"
      And the output should include "It crashes on startup"
      And the output should include "children"
      And the output should include "tags"

    Example: Nested yaks appear as recursive children in JSON
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      When I list the yaks as json
      Then the output should include "parent"
      And the output should include "child"

    Example: JSON includes tags
      Given I have a clean git repository
      And I add the yak "my yak"
      When I tag "my yak" with "v1"
      And I list the yaks as json
      Then the output should include "v1"

    Example: JSON reflects yak state
      Given I have a clean git repository
      And I add the yak "my yak"
      And I mark the yak "my yak" as done
      When I list the yaks as json
      Then the output should include "done"

  Rule: Invalid format and filter values are rejected

    Example: Unknown format is rejected
      Given I have a clean git repository
      When I try to list the yaks in "foobar" format
      Then the command should fail
      And the error should contain "Unknown format"
      And the error should contain "pretty"

    Example: Unknown filter is rejected
      Given I have a clean git repository
      When I try to list the yaks filtering by "foobar"
      Then the command should fail
      And the error should contain "Unknown filter"
      And the error should contain "done"
