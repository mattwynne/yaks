# 14. No optimistic concurrency on event appends

Date: 2026-02-28

## Status

accepted

## Context

In a typical event-sourced system, commands follow this pattern:

1. Load aggregate state from the event store (or a snapshot)
2. Validate business rules against current state
3. Emit domain events
4. Append events to the store

Step 4 usually includes an optimistic concurrency check: "append
these events only if the stream is still at the version I loaded
in step 1." If another process appended events between steps 1
and 4, the append is rejected and the caller retries with fresh
state. This prevents lost updates where two processes read the
same state, make independent decisions, and both succeed.

In `yx`, the `Application::with_yak_map_result_using_metadata`
method implements exactly this pattern without the concurrency
check:

```
let mut yak_map = YakMap::from_store(self.store, metadata)?;
let result = f(&mut yak_map)?;
self.save_yak_map(&mut yak_map)?;
```

`YakMap::from_store` loads state from the read model (the
`.yaks/` directory projection). The closure validates and emits
events. `save_yak_map` iterates the pending events and calls
`event_store.append()` for each one — with no "expected version"
guard.

The `EventStore::append` trait method accepts a single event and
has no parameter for expected stream position:

```rust
fn append(&mut self, event: &YakEvent) -> Result<()>;
```

`GitEventStore::append` creates a git commit on `refs/notes/yaks`
parented on whatever the current tip is. If two processes race,
both succeed; neither detects the other's writes.

### When this matters

Two `yx` processes running simultaneously against the same
repository could produce an inconsistent event log. For example:

- Process A reads state: yak "deploy" is `todo`
- Process B reads state: yak "deploy" is `todo`
- Process A marks "deploy" as `wip`, appends `FieldUpdated`
- Process B marks "deploy" as `done`, appends `FieldUpdated`
- Result: "deploy" was marked `done` without ever passing through
  the `wip` state in the log — and the business rule that checked
  "is this a valid state transition?" ran against stale state

Similarly, hierarchy rules (e.g., "cannot mark parent done if
children are incomplete") could be bypassed if concurrent
processes validate against different snapshots.

### Current mitigations

**Sequential CLI usage.** `yx` is a command-line tool. Normal
human usage is inherently sequential — one command completes
before the next begins. There is no daemon, server, or
concurrent request pipeline.

**CRDT-style sync merge.** When event logs diverge (during
`yx sync`), the sync mechanism replays events from both sides
and applies CRDT merge semantics (see ADR 0007). This means
even if concurrent appends create a diverged history, sync will
deterministically converge to a consistent state. The merge is
designed for multi-repository collaboration, but it also covers
the local concurrent-append case.

**Event-level idempotency.** Each event carries a unique
`Event-Id`. `GitEventStore::append` checks for duplicate
event IDs and skips them, preventing replay-based duplication
(though this does not help with the stale-state problem).

**No file-level locking.** There is no `flock`, lockfile, or
other OS-level concurrency control on the event store or the
`.yaks/` projection directory.

## Decision

Accept the absence of optimistic concurrency control on event
appends. Do not add a version check at this time.

The risk is low because:

1. `yx` is a single-user sequential CLI tool. The primary
   concurrency scenario (two humans typing `yx` commands in
   parallel against the same repo) is rare and low-stakes.

2. Agent-driven workflows (e.g., `pi` subagents) operate in
   separate git worktrees with their own `.yaks/` projections.
   They share the same `refs/notes/yaks` ref (since worktrees
   share the git object store), but the CRDT sync merge handles
   convergence when branches are reconciled.

3. Adding optimistic concurrency would require:
   - A stream version concept in the `EventStore` trait
   - A `load_version` return from `YakMap::from_store`
   - A conditional `append_if_version` method
   - Retry logic in `Application::with_yak_map`
   - Changes to both `GitEventStore` and `InMemoryEventStore`

   This is non-trivial complexity for a scenario that does not
   arise in practice today.

If `yx` evolves to support concurrent access (e.g., a daemon
mode, a web UI, or multi-agent parallel writes to the same
worktree), this decision should be revisited.

## Consequences

### What stays easy

- The `EventStore` trait remains simple: `append` is a
  straightforward unconditional write.
- No retry loops, no version tracking, no conditional writes.
- Use cases remain single-pass: load, decide, emit, save.

### What becomes harder

- If two processes do race against the same repo, the resulting
  event log may contain events that were validated against stale
  state. Business rule violations won't be caught at write time.
- The CRDT merge during sync will converge to a consistent
  state, but that state may not match what either process
  intended.
- Debugging such issues would be difficult since there is no
  error or warning at the point of conflict.

### Revisit triggers

- Introduction of a long-running daemon or server process
- Multiple agents writing to the same worktree concurrently
- User reports of state inconsistencies from parallel CLI usage
- Any move toward collaborative real-time editing
