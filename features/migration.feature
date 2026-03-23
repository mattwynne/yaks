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
