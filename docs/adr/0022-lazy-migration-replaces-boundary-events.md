# 22. Lazy Migration Replaces Boundary Events

Date: 2026-03-22

## Status

accepted (supersedes ADR 0012 Compact after migration to hide
pre-migration history)

## Context

ADR 0012 established that after running schema migrations, the
migrator creates a boundary commit (originally `Compacted`, later
`Migrated`) so that `get_all_events()` stops walking before it
reaches pre-migration commits with old blob formats. This worked
for the local case but caused data loss during sync.

### The incident

A machine with a stale local event store (old schema version,
old yaks) upgraded its binary and ran `yx sync`. The sequence:

1. Migration ran first (before sync), creating a `Migrated`
   boundary commit that captured the **stale local state** as
   snapshots with a **current timestamp**.
2. Sync fetched 824 correct events from origin and merged them
   with the local `Migrated` event.
3. The merge sorts events by timestamp. The `Migrated` event
   (timestamp "now") sorted **last** — after all the correct
   events.
4. During replay, all correct events were applied first, then the
   `Migrated` event cleared everything and replaced it with stale
   snapshots.
5. The stale state was pushed to origin, corrupting it for all
   peers.

The root cause: migration creates a boundary event that captures
local state, but local state may be arbitrarily stale. The merge
algorithm has no way to distinguish "fresh authoritative snapshot"
from "stale snapshot from a machine that hasn't synced in weeks."

### Why the Migrated event existed

ADR 0012's reasoning was sound for the local case: the read code
only understands the current schema format, so a boundary prevents
`get_all_events()` from walking into old-format commits. The
alternative — backward-compatible fallback paths in the reader —
was rejected because it would accumulate dead code with every
future migration.

### A third option

Every commit tree already carries `.schema-version` (also
established in ADR 0012). The reader can detect old-format trees
and apply the migration chain to transform them at read time.
This uses the **same migration code** that already exists for
upgrading refs — just invoked lazily on individual trees instead
of eagerly on the entire history.

## Decision

### Lazy migration at read time

`get_all_events()` checks `.schema-version` on each commit's
tree. If it's older than `CURRENT_SCHEMA_VERSION`, the migration
chain transforms the tree to current format before reading
snapshots or field content from it. Each migration implements a
`TreeMigration` trait that transforms an arbitrary tree OID
without committing.

### Migration upgrades the tip tree only

On startup, if the tip tree's schema version is behind, the
migrator still runs to upgrade the tip tree (so new event appends
use the current format). But it only calls
`write_schema_version()` — it does **not** create a boundary
commit.

### No compaction after migration

Migration and compaction are now separate concerns. Migration
upgrades the tip so writes are correct. Compaction remains a
user-triggered performance optimisation (`yx compact`). The
disk projection rebuild on startup (`needs_projection_reset`)
handles the first-run-after-upgrade read cost.

### Migrated events stripped from merge

For backward compatibility with `Migrated` commits already in
the wild (from older binaries), `merge_event_streams()` filters
out `YakEvent::Migrated` events before merging. They do not
participate in the merge and are not counted in pulled/pushed
totals.

Existing `Migrated` commits still function as read boundaries in
`get_all_events()` — the code still recognises them and reads
their trees. No new ones are created.

## Consequences

### What becomes easier

- **Sync is safe after migration**: No boundary event means no
  stale snapshot to corrupt the merge. The bug that prompted
  this change is eliminated at the root.
- **Migration is decoupled from compaction**: Changes to
  compaction semantics no longer need to account for migration.
- **Fallback code concern is addressed differently**: The
  migration chain serves as the fallback, invoked at read time.
  Same code, same maintenance burden as before — just called
  lazily instead of eagerly.

### What becomes harder

- **Reads are slower for old history**: Walking past old-format
  commits incurs migration cost per tree. Mitigated by the disk
  projection (which is what `yx ls` and most commands read) and
  by user-triggered compaction for stores with long histories.
- **Migration code must support tree-level operations**: Each
  migration now implements both `Migration` (ref-level, for
  startup tip upgrade) and `TreeMigration` (tree-level, for
  lazy reads). This is a small amount of additional code per
  migration.

### What is no longer open

- **ADR 0012's "Migration of Compacted trees from older
  schemas"**: Resolved. Lazy migration transforms any old-format
  tree on the fly, including Compacted trees from older schemas.
- **ADR 0020's "Zombie yak problem"**: The specific variant
  caused by Migrated events is eliminated. Other zombie yak
  scenarios (orphan reordering with Compacted events) remain
  as noted in ADR 0009.
