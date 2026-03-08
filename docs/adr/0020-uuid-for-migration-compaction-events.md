# 20. UUID for Migration Compaction Events

Date: 2026-03-07

## Status

accepted (amends ADR 0012 Compact after migration)

## Context

ADR 0012 established that the migrator creates a `Compacted` commit
after running schema migrations. The event ID for this commit was
deterministic — `migration-to-v{N}` — so that every repo migrating
to the same schema version would produce the same event ID.

This caused data loss when two repos independently migrated to the
same schema version. Each repo produced a `Compacted` event with
the same event ID (`migration-to-v7`) but different snapshot
content — because the repos had different local state at the time
of migration. During sync, the merge algorithm deduplicated by
event ID and silently discarded one snapshot.

In the observed incident, the main repo had 31 yaks (many had been
done and removed) while a stale clone had 42 yaks (still containing
yaks that had been removed on main). Both independently migrated
v5→v7 and produced `migration-to-v7` events. After sync, dedup
kept one and dropped the other, contributing to zombie yaks
reappearing.

## Decision

Use `Uuid::now_v7()` for migration compaction event IDs instead of
the deterministic `migration-to-v{N}` format.

Each independent migration produces a unique event ID, so the merge
algorithm treats them as separate events rather than deduplicating
one away.

## Consequences

### What becomes easier

- **Independent migration is safe**: Two repos can migrate to the
  same schema version without data loss from event ID collision.

### What becomes harder

- **Two Compacted events after sync**: When both peers independently
  migrate, the merged stream contains two `Compacted` events. The
  merge algorithm currently only handles the first `Compacted`
  event it finds (noted as an open edge case in ADR 0009). This
  needs further work.
- **No dedup shortcut**: The merge algorithm can no longer rely on
  event ID to recognise that two migrations represent the same
  logical operation.

### What remains open

- **Zombie yak problem persists**: This change prevents data loss
  from dedup, but the orphan reordering logic in
  `merge_event_streams()` can still resurrect removed yaks when
  merging stale peer events with a compacted stream. That is a
  separate issue (see: "merge doesn't handle Removed events before
  Compacted for snapshot yaks").
