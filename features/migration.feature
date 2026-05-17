Feature: Schema migration
  The event store in refs/notes/yaks has a schema version.
  When the binary expects a newer schema than what's stored,
  it runs migrations to bring the store up to date.

  Rule: Existing stores are migrated transparently

    @fullstack
    Example: Store created before schema versioning still works
      Given a yak "make tea" created with the v1 schema
      When I list the yaks
      Then the output should include "make tea"

  Rule: Migration does not produce a Migrated boundary event

    @fullstack
    Example: Migration does not produce a Migrated event in the log
      Given a yak "make tea" created with the v1 schema
      When I run yx log
      Then the output should not include "migrated"

  Rule: Legacy blocked state remains readable as a manual blocker

    @fullstack
    Example: Legacy stored blocked state is shown as todo with a manual blocker
      Given a yak "legacy blocked" created with legacy blocked state
      When I list the yaks as json
      Then the JSON yak "legacy blocked" should have state "todo"
      And the JSON yak "legacy blocked" should have ready false
      And the JSON yak "legacy blocked" should have exactly one manual blocker with reason "Migrated from blocked state"

    @fullstack
    Example: Historical blocked state events are understandable
      Given a yak "historically blocked" has a historical blocked state event
      When I list the yaks as json
      Then the JSON yak "historically blocked" should have state "todo"
      And the JSON yak "historically blocked" should have exactly one manual blocker with reason "Migrated from blocked state"
      When I show the log
      Then the output should include "changed state of historically blocked to blocked"

    @fullstack
    Example: Resetting and replaying legacy blocked yaks does not duplicate manual blockers
      Given a yak "legacy blocked" created with legacy blocked state
      When I reset the yaks
      And I reset the yaks from disk to git
      And I reset the yaks
      And I list the yaks as json
      Then the JSON yak "legacy blocked" should have state "todo"
      And the JSON yak "legacy blocked" should have ready false
      And the JSON yak "legacy blocked" should have exactly one manual blocker with reason "Migrated from blocked state"

    @fullstack
    Example: Compaction after migration stores todo rather than blocked
      Given a yak "legacy blocked" created with legacy blocked state
      When I list the yaks as json
      And I run yx compact --yes
      Then the git event store should not contain a blocked state field
      When I list the yaks as json
      Then the JSON yak "legacy blocked" should have state "todo"
      And the JSON yak "legacy blocked" should have exactly one manual blocker with reason "Migrated from blocked state"
