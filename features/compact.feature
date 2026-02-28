@fullstack
Feature: yx compact - Compact the event stream

  Compacting replaces the full event history with a snapshot,
  reducing the size of the event store. After compaction, all
  existing yaks are preserved but the individual history events
  are replaced by snapshot events following a Compacted marker.

  See also:
    - docs/adr/0009-compacted-event-design-and-known-gaps.md
    - docs/adr/0010-state-reconstruction-mechanisms.md

  Background:
    Given I have a clean git repository

  Rule: Compacting preserves all yaks

    Example: Yaks survive compaction
      Given I add the yak "make the tea"
      And I add the yak "buy biscuits" under "make the tea"
      When I set the state of "buy biscuits" to "wip"
      And I run yx compact --yes
      Then it should succeed
      And the output should include "Compacted event stream"
      When I list the yaks in "markdown" format
      Then the output should be:
        """
        - [wip] make the tea
          - [wip] buy biscuits
        """

  Rule: Log shows snapshot events nested under Compacted marker

    Example: Snapshot events appear indented under the Compacted event
      Given I add the yak "make the tea"
      When I set the state of "make the tea" to "wip"
      And I run yx compact --yes
      Then it should succeed
      When I run yx log
      Then the output should include "Compacted"
      And the output should include "        Added:"
      And the output should include "        FieldUpdated:"
      And the output should not include "event -"

  Rule: New events work after compaction

    Example: Adding a yak after compacting
      Given I add the yak "make the tea"
      When I run yx compact --yes
      Then it should succeed
      When I add the yak "buy biscuits"
      And I list the yaks in "markdown" format
      Then the output should be:
        """
        - [todo] buy biscuits
        - [todo] make the tea
        """

  Rule: User can abort compaction

    Example: No --yes flag aborts when stdin is not a TTY
      Given I add the yak "make the tea"
      When I run yx compact
      Then it should succeed
      When I run yx log
      Then the output should not include "Compacted"
