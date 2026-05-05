Feature: Focus yaks with YX_FOCUS
  YX_FOCUS narrows yx commands to one yak, its ancestors, and its descendants.

  Background:
    Given I have a clean git repository
    And I add the yak "Project" with id "project"
    And I add the yak "A" with id "a" under "Project"
    And I add the yak "B" with id "b" under "A"
    And I add the yak "C" with id "c" under "A"
    And I add the yak "D" with id "d" under "Project"
    And I add the yak "E" under "B"
    And I add the yak "F" under "B"

  Rule: Listing is pruned to the focus

    Example: Pretty list shows pruned ancestor branches with markers
      Given YX_FOCUS is set to "b"
      When I list the yaks
      Then the output should include "┆"
      And the output should include "Project"
      And the output should include "A"
      And the output should include "B"
      And the output should include "E"
      And the output should include "F"
      And the output should not include "C"
      And the output should not include "D"

  Rule: Commands cannot target hidden yaks

    Example: Hidden yaks report that they are outside YX_FOCUS
      Given YX_FOCUS is set to "b"
      When I try to mark the yak "D" as done
      Then the command should fail
      And the error should contain "outside YX_FOCUS"

  Rule: New rootless yaks are added under the focus

    Example: Add without --under uses the focused yak as parent
      Given YX_FOCUS is set to "b"
      When I add the yak "G"
      And I list the yaks in "plain" format
      Then the output should include "Project/A/B/G"
