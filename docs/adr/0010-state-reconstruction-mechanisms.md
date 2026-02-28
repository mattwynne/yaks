# 10. State Reconstruction Mechanisms

Date: 2026-02-26

## Status

accepted (supersedes the draft ADR 0010 referenced in ADR 0009)

## Context

The system has three mechanisms for reconstructing state. ADR 0009
noted that there were multiple overlapping mechanisms and
referenced an ADR 0010 that was drafted but never committed. Since
then, the soft `yx reset --git-from-disk` (which created non-event
snapshot commits invisible to sync) has been removed.

This ADR documents the current, simplified set of mechanisms and
their relationships to the event model.

## Decision

Three state reconstruction mechanisms exist, each with a distinct
purpose and clear event model participation:

### `yx reset` (disk from git)

Rebuilds the `.yaks/` directory from the git tree at HEAD of
`refs/notes/yaks`. Clears the disk projection and replays from
the tree using `read_snapshots_from_tree()`.

**Purpose:** Recover a corrupted `.yaks/` directory.

**Event model:** Read-only. Does not write to the event store.

See `features/reset.feature`, rules "Reset rebuilds yaks from
the git event store tree" through "Reset only affects yak
entries".

### `yx reset --git-from-disk` (rebuild git from disk)

Wipes `refs/notes/yaks` entirely, reads yaks from `.yaks/`,
and replays each through the `AddYak` use case. This produces
genuine event commits with proper metadata.

**Purpose:** Get a clean event history when the git event store
is corrupted or needs rebuilding. Requires force-pushing to
shared remotes and collaborators must force-fetch.

**Event model:** Full participation. Every yak becomes a
sequence of real events (Added, FieldUpdated, etc.).

See `features/reset.feature`, rule "Reset from disk replays
yaks through the Application layer".

### `yx compact` (checkpoint event stream)

Creates a `Compacted` domain event carrying `Vec<YakSnapshot>`,
appended to the event store. Future `get_all_events()` calls
stop at the Compacted commit and reconstruct state from its
tree.

**Purpose:** Reduce event store size for performance while
preserving the event model.

**Event model:** Full participation. The Compacted event is a
domain event with an `event_id`, participates in sync merges,
and the projection knows how to apply it.

See `features/compact.feature` and ADR 0009 for the Compacted
event design.

### What was removed

The soft `yx reset --git-from-disk` (without `--hard`) previously
created a single snapshot commit with message "Snapshot: rebuilt
from disk". This commit was not a valid event — `get_all_events()`
would skip it via an `Err(_) => continue` fallback. It was
invisible to sync and violated event sourcing principles. It has
been removed, and `--git-from-disk` now does what `--hard` did.

## Consequences

### What becomes easier

- **Fewer code paths:** Two mechanisms write to the event store
  (reset --git-from-disk and compact), both producing valid
  events. No more non-event commits in the git history.
- **Clearer mental model:** Each mechanism has a single purpose
  with no overlap. Reset rebuilds one direction or the other;
  compact checkpoints the stream.
- **Sync safety:** All git writes are valid events, so sync
  can reason about the full history.

### What remains open

- Whether `yx reset --git-from-disk` and `yx compact` could be
  unified. Both say "here is the current state, make it the new
  baseline" but differ in destructiveness (compact preserves
  pre-compaction history; reset wipes it).
- Whether the default `yx reset` should preserve yak IDs in all
  cases. It currently uses `read_snapshots_from_tree()` which
  preserves IDs, but legacy migration scenarios may still exist.
