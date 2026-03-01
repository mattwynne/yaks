Feature: Mark yaks as done
  Marks yaks as completed, changing their state to "done".
  Done yaks stay visible in listings for context. Use prune to
  remove them. Use `yx state <name> todo` to reopen a completed yak.

  Rule: A yak can be marked as done
    Changes the yak's state to "done" and it appears grayed out in listings.

    Example: Marking a simple yak as done
      Given I have a clean git repository
      And I add the yak "Fix the bug"
      When I mark the yak "Fix the bug" as done
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [done] Fix the bug
        """

  Rule: Children can be marked done independently
    Marking a child as done does not affect siblings. The parent
    auto-transitions to "wip" when it has children.

    Example: Marking a nested child as done
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      When I mark the yak "child" as done
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] parent
          - [done] child
        """

  Rule: Cannot mark a parent as done while children are incomplete
    A parent with incomplete children must have its children completed
    first, or use --recursive to mark the entire subtree.

    Example: Error when parent has incomplete children
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child" under "parent"
      When I try to mark the yak "parent" as done
      Then the command should fail
      And the error should contain "cannot mark 'parent' as done - it has incomplete children"

  Rule: Recursive flag marks entire subtree as done
    The --recursive flag marks a yak and all its descendants as done,
    regardless of depth.

    Example: Recursive done marks parent and all descendants
      Given I have a clean git repository
      And I add the yak "parent"
      And I add the yak "child1" under "parent"
      And I add the yak "child2" under "parent"
      And I add the yak "grandchild" under "child1"
      When I mark the yak "parent" as done recursively
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [done] parent
          - [done] child1
            - [done] grandchild
          - [done] child2
        """
