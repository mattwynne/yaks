# 9. Compacted Event Design

Date: 2026-02-26

## Status

accepted

## Context

As the event store grows, replaying the full history becomes
expensive. We needed a way to checkpoint the event stream —
replacing old events with a snapshot of current state.

### The synthetic events approach (and why we moved away)

The first attempt at compaction read the git tree and synthesized
fake `Added` and `FieldUpdated` events — events that never
actually happened, with fabricated metadata. The projection would
replay these to rebuild state. This was fragile:

- **Dishonest**: The event stream contained events that were
  manufactured, not recorded facts. This violated the core
  event sourcing principle that events are immutable historical
  records.
- **Lossy**: Synthesizing events from tree state required
  choosing what to emit. Custom fields, metadata authorship,
  and creation timestamps had to be reverse-engineered from
  blobs, with fallbacks for missing data.
- **Duplicated code paths**: `snapshot_events()` had its own
  200-line tree-walking implementation, separate from
  `read_snapshots_from_tree()` which did nearly the same work
  for other purposes. Changes to the tree format had to be
  made in both places.

### Problems with the git tree after compaction

The compacted tree was built from `treebuilder(None)` —
a blank slate containing only yak subtrees. It did not carry
forward the `.schema-version` blob, so after compaction the
migrator would see schema v1 and unnecessarily re-run all
migrations.

### Sync data loss risk

The CRDT-style merge algorithm treated all events as additive —
dedup by event_id, sort by timestamp. But compaction has
destructive semantics: the projection calls `clear_all()` then
rebuilds from the snapshot. If a peer had unsynced events
timestamped before the compaction, they would sort before the
`Compacted` event and be wiped by `clear_all()` during replay.

Example: Alice compacts at T=100 (snapshot contains yaks A, B, C).
Bob created yak D at T=80 while offline. After sync, the merged
stream is `[Added(D) T=80, Compacted T=100, ...]`. Replaying
this builds D, then Compacted wipes it. D is lost.

### Multiple state reconstruction mechanisms

The codebase had accumulated several ways to rebuild state
(`yx reset`, `yx reset --git-from-disk`, `yx reset --hard`,
`yx compact`), each with a different relationship to the event
model. See ADR 0010 for the full inventory.

## Decision

### Compacted is a domain event carrying snapshot data

Add a `Compacted(Vec<YakSnapshot>, EventMetadata)` variant to
the event enum. `YakSnapshot` captures the full state of a yak
(id, name, parent_id, state, context, custom fields, author,
timestamp). The projection applies it by clearing and rebuilding.

This replaces the synthetic events approach: instead of
pretending events happened, we honestly say "here is the state
at this point."

### One canonical tree reader

`read_snapshots_from_tree()` is the single method for reading
yak state from a git tree. Both `get_all_events()` (for
Compacted commits) and `snapshot_events()` (for `yx reset`)
delegate to it. The old `collect_snapshot_events()` with its
duplicated tree-walking is removed.

### Git tree is the payload, commit message is a label

In the git store, the commit message is just `"Compacted"`.
The snapshot data is the git tree itself. `format_message()` →
`parse()` is intentionally lossy for this event type — the
in-memory store carries snapshots directly, the git store
reads them from the tree.

### Compacted tree includes .schema-version

The tree built for a Compacted event includes `.schema-version`
set to `CURRENT_SCHEMA_VERSION`, preventing unnecessary
migration runs after compaction.

### Merge algorithm treats Compacted as a checkpoint

After the initial sort-by-timestamp, the merge detects events
that pre-date a Compacted event but affect yak IDs not in its
snapshot. These "orphaned" events are moved after the Compacted
event in replay order, so they're applied on top of the snapshot
state rather than being wiped by `clear_all()`.

## Consequences

### What becomes easier

- **Safe compaction in collaborative workflows**: Unsynced peer
  events survive compaction. The merge algorithm preserves them.
- **Single tree-reading path**: Changes to the tree format only
  need updating in one place.
- **Honest event stream**: No more synthetic events. Compacted
  says what it is.

### What remains harder

- **Merge complexity**: The merge algorithm has Compacted-specific
  orphan detection and reordering logic.
- **Multiple Compacted events**: If both peers compact
  independently, the merge stream could contain two Compacted
  events. The current implementation handles the first one found.
  This edge case hasn't been tested extensively.
- **Lossy format asymmetry**: Code that assumes `format_message()`
  → `parse()` is lossless will silently lose Compacted snapshot
  data. Documented but could surprise future contributors.
