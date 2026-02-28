# 11. Schema Versioning and Sync Compatibility

Date: 2026-02-28

## Status

accepted (supersedes the open question on event schema evolution
in ADR 0002)

## Context

ADR 0002 noted as an open question: "How to handle breaking changes
to event schemas as the domain model evolves." Since then, a schema
versioning and migration system has been built:

- A `.schema-version` blob in the git tree at `refs/notes/yaks`
  tracks the current schema version.
- A `Migrator` runs on startup, checking the local ref's version
  against `CURRENT_SCHEMA_VERSION` and running any pending
  migrations (v1→v2, v2→v3, etc.).
- Each migration transforms the tree structure from one version to
  the next.

This works well for the local event store. However, sync introduces
a second ref: the peer ref fetched from the remote into
`refs/notes/yaks-peer`. The migrator only runs on the local ref.
The peer ref is read as-is.

This creates two risks:

**Newer peer, older binary.** Alice upgrades her binary (which
migrates her local store to v5 and changes blob names), syncs to
origin. Bob, still on v4, fetches Alice's ref. His code reads the
v5 tree but doesn't recognise the new blob names, silently losing
data (e.g., author metadata).

**Older peer, newer binary.** Bob upgrades to v5, fetches Alice's
v4 ref. His code expects the v5 tree format but the peer tree is
still v4. If the migration changed structure (flattened nesting,
renamed blobs), the read code can't find what it expects.

Both risks stem from the same design gap: the code assumes the
tree it reads is at the current schema version, but the peer ref
has never been migrated.

## Decision

### One format, enforced by migration

The code reads only the current schema version. There are no
backward-compatible read paths and no fallback logic for old
formats. Migrations are the sole mechanism for transforming old
formats to current.

### Migrate the peer ref before reading

During sync, after fetching the peer ref, run the migrator on it
before reading any events. This reuses the existing migration
infrastructure — `Migrator` and `EventStoreLocation` already
support arbitrary ref names.

### Refuse if peer is newer

If the peer ref's schema version is greater than
`CURRENT_SCHEMA_VERSION`, sync refuses with a "please update yx"
error. The peer ref is cleaned up before bailing.

### Sync schema compatibility flow

```
1. Fetch peer ref from origin
2. Read peer schema version
3. If peer > local binary  → bail ("please update yx")
4. If peer < local binary  → migrate peer ref to current
5. If peer == local binary → no-op
6. Proceed with merge
```

## Consequences

### What becomes easier

- **No backward-compatible read code.** Every read path assumes
  the current schema version. Migrations handle the rest.
- **Safe schema changes.** Future migrations (renaming blobs,
  restructuring trees) cannot cause silent data loss during sync.
- **Reuse of existing infrastructure.** The `Migrator` and
  `Migration` trait already work on arbitrary refs. No new
  abstractions needed.

### What becomes harder

- **Migration must be idempotent on peer refs.** Running migration
  on a temporary ref that gets deleted afterward is fine, but the
  migration code must handle the peer ref cleanly (no assumptions
  about ref name).
- **Both directions block on version mismatch.** If Alice is at v5
  and Bob at v4, Bob cannot sync until he upgrades. This is
  intentional — silent degradation is worse than a clear error.

### What remains open

- **Migrating old peer Compacted trees.** When the peer ref
  contains a Compacted commit from a previous schema version, the
  migration must also transform the Compacted tree. Existing
  migrations only transform the tip tree. This may need extending
  for migrations that change blob names inside yak subtrees.
