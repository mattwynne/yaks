Feature: Prune done yaks
  Removes all completed yaks in bulk. Confirms how many yaks were
  pruned on success (exit code 0). Use prune for bulk cleanup after
  completing a sprint or milestone. Use rm to remove a specific yak.

  Rule: Removes all done yaks and keeps incomplete yaks

    Each yak is evaluated independently. Any yak with state "done" is
    removed. All other yaks are kept.

    Example: Prune removes done yaks and keeps todo yaks
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      And I add the yak "Write docs"
      And I mark the yak "Fix the bug" as done
      When I prune done yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] Write docs
        """

  Rule: Done child yaks are removed independently of parent state

    A done child is removed even if its parent is not done. A non-done
    child is kept even if a sibling was pruned.

    Example: Prune removes done child but keeps non-done siblings
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child1" under "parent"
      And I add the yak "child2" under "parent"
      And I mark the yak "child1" as done
      When I prune done yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] parent
          - [todo] child2
        """

    Example: Prune removes done parent when all children are also done
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      And I mark the yak "child" as done
      And I mark the yak "parent" as done
      When I prune done yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        You have no yaks. Are you done?
        """

  Rule: Exclude-tag preserves matching done yaks from pruning

    When --exclude-tag is passed, done yaks carrying that tag survive
    pruning. Done yaks without the tag are still removed. Without
    the flag, all done yaks are pruned as before (even tagged ones).

    Example: Done yak with excluded tag is kept
      Given I have a clean git repository
      And I add the yak "important"
      And I tag "important" with "keep"
      And I mark the yak "important" as done
      And I add the yak "old stuff"
      And I mark the yak "old stuff" as done
      When I prune done yaks excluding tag "keep"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [done] important @keep
        """

    Example: Tagged done yak pruned when no flag given
      Given I have a clean git repository
      And I add the yak "important"
      And I tag "important" with "keep"
      And I mark the yak "important" as done
      When I prune done yaks
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        You have no yaks. Are you done?
        """

    Example: Excluded parent with done untagged child
      Given I have a clean git repository
      And I add the yak "epic"
      And I tag "epic" with "keep"
      And I add the yak "subtask" under "epic"
      And I mark the yak "subtask" as done
      And I mark the yak "epic" as done
      When I prune done yaks excluding tag "keep"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [done] epic @keep
        """

    Example: Untagged done parent with excluded child
      Given I have a clean git repository
      And I add the yak "wrapper"
      And I add the yak "important" under "wrapper"
      And I tag "important" with "keep"
      And I mark the yak "important" as done
      And I mark the yak "wrapper" as done
      When I prune done yaks excluding tag "keep"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [done] wrapper
          - [done] important @keep
        """
