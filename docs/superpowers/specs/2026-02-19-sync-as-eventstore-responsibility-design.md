# Design: Sync as EventStore Responsibility

Date: 2026-02-19
ADR: 0007

## Problem

`refs/notes/yaks` has two writers with incompatible formats:
GitEventStore uses yak IDs as tree entry keys; GitRefSync builds
trees from `.yaks/` where DirectoryStorage uses slugified names.
This causes duplicates on sync when migration renames entries.
See ADR 0007 for full root cause analysis.

## Design

### Sync moves into the EventStore

The `EventStore` trait gains a `sync()` method. Sync is
peer-to-peer: one event store syncs directly with another.

```
EventStore.sync(peer, event_bus) -> SyncStatus
```

- **peer**: another event store to sync with (git repo path,
  or in-memory store reference)
- **event_bus**: receives newly-synced events so projections
  update
- **SyncStatus**: NoOp | Pushed | Pulled | Merged

### Event IDs

Each event gets an `event_id` field:
- Git implementation: commit SHA
- In-memory: UUID or sequential counter

Event IDs enable:
- **Idempotent append**: EventStore skips events it already has
- **Deduplication during sync**: merge by event_id set union
- **Incremental tracking**: record last-synced event_id

### Sync flow (GitEventStore)

1. Record current HEAD of `refs/notes/yaks`
2. Fetch peer's `refs/notes/yaks`
3. Merge (fast-forward when possible, tree merge otherwise)
4. Walk commits from old HEAD to new HEAD, parse new events
5. Publish new events through EventBus:
   - EventStore receives them, checks event_id, no-ops
     (already stored via git merge)
   - YakStore receives them, updates `.yaks/` projection
6. Push merged ref back to peer
7. Update `refs/notes/yaks-synced` tracking ref

### Sync flow (InMemoryEventStore)

1. Compare event lists by event_id
2. Copy missing events from peer to local (and vice versa)
3. Publish new events through EventBus

### Peer-to-peer model

Both git and in-memory use the same mental model: two stores
exchange events directly.

- **Git**: `git fetch /path/to/peer refs/notes/yaks` — works
  for remotes, local repos, and worktrees
- **In-memory**: direct event list merge between two stores
- **Contract tests**: create two stores, mutate independently,
  sync, verify both see all events

### .yaks/ is a one-way projection

Sync never reads from `.yaks/`. The YakStore projection is
updated by events flowing through the EventBus after sync.
A future `yx commit` command may capture manual filesystem
edits back into the event store, but that is out of scope.

## What gets removed

- `SyncPort` trait
- `GitRefSync` adapter
- `merge_remote_into_local_yaks()` — source of the duplicates
- `build_tree_from_yaks()` — no longer reads `.yaks/` for sync

## Incremental delivery

1. Add `sync()` to EventStore trait, implement in-memory
   double, add contract tests
2. Implement git-native sync in GitEventStore
3. Wire `SyncYaks` use case to `EventStore.sync()`, extract
   `.yaks/` projection via EventBus
4. Remove `SyncPort` and `GitRefSync`
5. (Future) CRDT-style event replay for smarter conflict
   resolution
