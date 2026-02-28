# 12. Compact After Migration to Hide Pre-Migration History

Date: 2026-02-27

## Status

accepted (extends ADR 0009 Compacted event design, ADR 0011 Schema
versioning and sync compatibility)

## Context

ADR 0011 established that migrations transform the git tree from
one schema version to the next (e.g., renaming `.metadata.json` to
`.created.json` in v4→v5). The migration commits a new tree with
the updated structure, and the read code only understands the
current format — no backward-compatible fallback paths.

This created a problem: **`get_all_events()` walks the full commit
history**, not just the tip tree. After a migration renames blobs,
older commits still contain the old blob names. The read code,
which only knows about `.created.json`, silently loses author and
timestamp data when it encounters a pre-migration commit that has
`.metadata.json`.

The initial v4→v5 migration added fallback code — "read
`.created.json`, falling back to `.metadata.json`" — in both the
git event store and the directory store. This worked but violated
the principle from ADR 0011 that the code reads only the current
schema version. Every future migration that renamed or restructured
blobs would need its own fallback path, accumulating dead code
forever.

A second problem: the disk projection (`.yaks/` directory) is not
rebuilt by migration. After v4→v5 migrates the git tree, the
`.yaks/` directory still contains `.metadata.json` files. The
directory store's `read_metadata()` also needed a fallback, and
any code reading `.yaks/` directly would see stale files.

A third problem: fresh event stores created by tests or direct API
use had no `.schema-version` blob. The migrator would see `None`
for a brand new ref and skip migrations — correct — but the first
event committed would have no version stamp. If that store was
later synced, the peer would see schema version 1 and attempt
unnecessary migrations.

## Decision

### Compact after migration

After running all pending migrations, the migrator creates a
`Compacted` commit on the ref. This commit:

- Has commit message `"Compacted\n\nEvent-Id: migration-to-vN"`
- Has the same tree as the migration tip (with `.schema-version`
  stamped)
- Is a child of the last migration commit

Since `get_all_events()` stops walking at `Compacted` commits
(see ADR 0009), pre-migration history with old blob names is
never visited. The read code only sees the compacted snapshot
tree, which is in the current format.

This eliminates all backward-compatible fallback code. The read
paths only know about `.created.json` — no `.metadata.json`
fallback in the git store or directory store.

### Rebuild disk projection after migration

`Migrator::run()` returns `bool` — `true` if migrations were
performed. When `main.rs` sees `true`, it rebuilds the disk
projection by calling `event_bus.rebuild()` with the events from
the compacted store. This clears old files (e.g., `.metadata.json`)
from `.yaks/` and writes the current format.

### Stamp `.schema-version` on every commit

The `append()` method in `GitEventStore` now ensures every commit
tree includes `.schema-version` set to `CURRENT_SCHEMA_VERSION`.
If the tree built from the event doesn't already have it, `append`
inserts it. This means:

- Fresh stores created by tests are properly versioned from the
  first commit
- The migrator never misidentifies a test store as v1
- Peer refs fetched during sync always have a discoverable version

## Consequences

### What becomes easier

- **No fallback code**: Read paths are simple — one blob name,
  one format. Future migrations that rename blobs don't need
  parallel read logic for old names.
- **Clean disk projection**: After migration, `.yaks/` is
  rebuilt from the compacted store. No stale files from previous
  schema versions.
- **Reliable schema detection**: Every commit carries its schema
  version. No ambiguity about whether a store is v1 or just
  unversioned.
- **Migration is a clean checkpoint**: The compacted commit serves
  as both a migration boundary and a performance optimization.
  Post-migration, the event store is always compact.

### What becomes harder

- **Compaction is now coupled to migration**: The migrator creates
  compacted commits, which means migration has a dependency on
  the compaction format. Changes to how `Compacted` works must
  account for the migration path.
- **Startup cost after upgrade**: The first run after a schema
  upgrade rebuilds the entire disk projection. For large stores,
  this adds latency to the first command after upgrading.
- **Every commit is slightly larger**: The `.schema-version` blob
  is stamped on every tree, adding ~10 bytes per commit. This is
  negligible but visible in `git log --raw`.

### What remains open

- **Migration of Compacted trees from older schemas**: If a
  pre-migration Compacted commit exists in the history, the new
  post-migration Compacted commit supersedes it. But if the peer
  ref has a Compacted commit from an older schema, migrating the
  peer ref must also handle its Compacted tree correctly (noted
  in ADR 0011).
- **Multiple upgrade jumps**: Running v1→v5 creates migration
  commits for each step (v1→v2, v2→v3, etc.) then one Compacted
  commit at the end. The intermediate commits are invisible to
  `get_all_events()` but remain in the git object store until
  `git gc`.
