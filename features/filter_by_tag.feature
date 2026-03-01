Feature: Filter list by tag

  Rule: Single tag filter
    `--tag foo` shows only yaks tagged `foo`, hiding everything else.

    Example: Shows yak with matching tag, hides others
      Given I have a clean git repository
      And I add the yak "deploy"
      And I tag "deploy" with "epic"
      And I add the yak "fix bug"
      And I tag "fix bug" with "urgent"
      When I list the yaks in "plain" format filtering by tag "epic"
      Then the output should include "deploy"
      But the output should not include "fix bug"

    Example: Hides untagged yaks
      Given I have a clean git repository
      And I add the yak "cleanup"
      When I list the yaks in "plain" format filtering by tag "epic"
      Then the output should not include "cleanup"

  Rule: Combines with --only
    `--tag` and `--only` are ANDed.

    Example: --tag epic --only done shows only done yaks with epic tag
      Given I have a clean git repository
      And I add the yak "A"
      And I tag "A" with "epic"
      And I mark the yak "A" as done
      And I add the yak "B"
      And I tag "B" with "epic"
      When I list the yaks in "plain" format filtering by tag "epic" and only "done"
      Then the output should include "A"
      But the output should not include "B"

  Rule: Tag normalization
    The @ prefix is stripped, same as yx tag add.

    Example: --tag @epic treated same as --tag epic
      Given I have a clean git repository
      And I add the yak "A"
      And I tag "A" with "epic"
      When I list the yaks in "plain" format filtering by tag "@epic"
      Then the output should include "A"

  Rule: No matches
    Empty result when nothing matches.

    Example: JSON format returns empty array
      Given I have a clean git repository
      And I add the yak "A"
      And I tag "A" with "urgent"
      When I list the yaks in "json" format filtering by tag "epic"
      Then the output should be:
        """
        []
        """

  Rule: IDs format
    `--format ids` outputs just yak IDs, one per line.

    Example: Lists yak IDs one per line
      Given I have a clean git repository
      And I add the yak "deploy" with id "deploy-1234"
      When I list the yaks in "ids" format
      Then the output should include "deploy-1234"

    Example: Combined with --tag for filtered IDs
      Given I have a clean git repository
      And I add the yak "deploy" with id "deploy-1234"
      And I tag "deploy" with "epic"
      And I add the yak "cleanup" with id "cleanup-5678"
      When I list the yaks in "ids" format filtering by tag "epic"
      Then the output should include "deploy-1234"
      But the output should not include "cleanup-5678"
