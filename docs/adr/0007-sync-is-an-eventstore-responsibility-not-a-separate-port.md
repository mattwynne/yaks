# 7. Sync is an EventStore responsibility, not a separate port

Date: 2026-02-19

## Status

accepted

## Context

Sync is currently implemented as a separate port (`SyncPort`) with
its own adapter (`GitRefSync`). This adapter reads the `.yaks/`
directory to build git trees, commits them to `refs/notes/yaks`,
merges with a remote ref, then extracts the result back to `.yaks/`.

This design has a fundamental flaw: two writers use incompatible
naming conventions for the same git ref.

- **GitEventStore** writes tree entries keyed by yak ID
  (e.g., `fix-the-tests-a1b2/`), flat structure, schema v4.
- **GitRefSync** builds trees from `.yaks/` where DirectoryStorage
  uses `slugify(name)` as directory names (e.g., `fix-the-tests/`),
  with nested hierarchy for parent/child relationships.

This causes a vicious cycle: the event store commits an id-keyed
tree, sync sees the slug-keyed `.yaks/` as "uncommitted changes"
and overwrites the ref with a slug-keyed tree, migration converts
it back to id-keyed, and the cycle repeats. When a remote is
involved, the merge combines slug-named and id-named entries,
creating duplicates of the same yak.

The root problem is that sync treats `.yaks/` as a two-way
synchronisation surface — both reading from it and writing to it —
when it should be a one-way projection of the event store.

## Decision

Sync becomes a responsibility of the EventStore, not a separate
port.

- **Remove `SyncPort`** and the `GitRefSync` adapter.
- **`GitEventStore` gains a `sync()` method** that synchronises
  its local `refs/notes/yaks` ref with a remote.
- **Git-native operations for the happy path**: fetch, fast-forward,
  and push. Since the event store IS a git ref, git can handle most
  sync scenarios natively.
- **CRDT-style event replay for conflicts**: when both sides have
  diverged and git tree merge produces conflicts, walk both commit
  histories back to the common ancestor, collect events from each
  side, and replay the merged event stream to build the final state.
- **`.yaks/` is a one-way projection**: sync never reads from
  `.yaks/`. After sync completes, the updated ref is extracted to
  `.yaks/` for human consumption. A future `yx commit` command may
  capture manual edits from `.yaks/` back into the event store, but
  that is out of scope for now.

## Consequences

**What becomes easier:**

- No more naming mismatch between event store and sync — there is
  only one writer (`GitEventStore`) with one format.
- No more migration-triggered duplicates on sync.
- Sync logic is simpler: git-native operations cover most cases.
- Conflict resolution can be smarter, using event semantics rather
  than file-level last-write-wins.
- Removing a port reduces the number of abstractions to maintain.

**What becomes harder or changes:**

- Manual edits to `.yaks/` files are not picked up by sync. Users
  must use `yx` commands to modify yaks. A `yx commit` command can
  be added later if needed.
- The EventStore port interface grows to include sync
  responsibility. The in-memory test double must implement
  sync behaviour so that contract tests can enforce sync
  semantics across all implementations.
- The CRDT-style conflict resolution is more complex to implement
  than the current directory-level merge, but produces correct
  results.
