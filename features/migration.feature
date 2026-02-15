Feature: Schema migration
  The event store in refs/notes/yaks has a schema version.
  When the binary expects a newer schema than what's stored,
  it runs migrations to bring the store up to date.

  Rule: Existing stores are migrated transparently

    Example: Store created before schema versioning still works
      Given a yak "make tea" created with the v1 schema
      When I list the yaks
      Then the output should include "make tea"
