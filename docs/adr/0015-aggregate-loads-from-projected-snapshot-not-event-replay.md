# 15. Aggregate Loads from Projected Snapshot, Not Event Replay

Date: 2026-02-28

## Status

accepted (elaborates on ADR 0002 "Aggregate loading cost" and ADR 0002 "Snapshot version linkage")

## Context

ADR 0002 adopted CQRS with Event Sourcing and noted in its
consequences that `YakMap::from_store()` loads the aggregate from
the read model (the `.yaks/` directory projection) rather than by
replaying events from the event store. It described this as
"effectively loading from a snapshot" and flagged the missing
version linkage as an open question.

This is a significant architectural trade-off that deserves its
own explicit record, because it means the system is not purely
event-sourced on the command side — the aggregate is
snapshot-sourced from a projected view.

### How it works today

Every command follows this flow:

1. `Application::with_yak_map()` calls `YakMap::from_store(store, metadata)`
2. `from_store` calls `store.list_yaks()` on the `ReadYakStore`
   (the filesystem projection under `.yaks/`)
3. It builds an in-memory `HashMap<YakId, YakState>` from that
   listing — names, parent IDs, states, and contexts
4. The use case mutates the `YakMap`, producing pending events
5. Events are persisted to the `EventStore` and applied to the
   projection

The aggregate never reads from the event store. It has no
`apply(event)` method. It does not know which events have been
applied, nor at what sequence number the snapshot was taken.

### Why this matters

In a textbook event-sourced system, the aggregate is
reconstituted by replaying events. This guarantees that the
aggregate's state is derived from the authoritative source of
truth (the event log). The current design inverts this: the
aggregate trusts the projection, which is a derived view.

## Decision

Accept and document this trade-off explicitly. The aggregate
loads from the projected read model (an informal snapshot) rather
than from event replay. This is a deliberate choice, not an
accidental omission.

### Rationale for the current approach

1. **Simplicity.** The `YakMap` aggregate is a straightforward
   in-memory data structure. It does not need an event-replay
   mechanism, upcasting logic, or schema migration during load.
   This keeps the codebase smaller and easier to understand.

2. **Performance.** Loading from the filesystem scales with the
   number of yaks (the current state), not the number of
   historical events. For a growing event log, this avoids
   replay cost entirely.

3. **The event store is append-only and co-located.** Events
   are persisted and the projection is updated synchronously in
   the same process. There is no async projection lag, no
   separate projection service, and no network boundary. The
   window for the projection to diverge from the event store is
   extremely small — limited to a crash between event persist
   and projection write.

4. **Recovery mechanisms exist.** `yx reset` rebuilds the
   `.yaks/` directory from the git event store tree (see ADR
   0010). This provides a manual recovery path if the projection
   becomes corrupt.

### Known implications and risks

1. **Projection bugs corrupt the aggregate.** If the
   `WriteToYakStore` projection has a bug — writing incorrect
   state, dropping a field, or mishandling an event — then
   `from_store` loads that corrupt state into the aggregate.
   Subsequent commands operate on the wrong baseline, and the
   events they emit encode decisions made from faulty premises.
   The event log remains technically correct (events were
   emitted honestly) but the decisions behind those events were
   based on corrupt state.

2. **No sequence number or version link.** The `.yaks/`
   directory has no marker indicating "this projection reflects
   events up to sequence N." If the projection and event store
   drift apart (e.g., a crash after event persist but before
   projection write, or a bug in one of the reset/compact
   paths), there is no way to detect the inconsistency
   automatically. The system silently loads stale or
   inconsistent state.

3. **The aggregate cannot detect missed events.** Because
   `from_store` does not consult the event store at all, it has
   no way to know whether the projection is behind, ahead, or
   diverged. A traditional event-sourced aggregate with a
   version number would fail fast on a version mismatch.

4. **Rebuilding from events is a separate code path.** The
   `yx reset` command and `yx compact` command reconstruct state
   from events, but they use different mechanisms
   (`read_snapshots_from_tree`, `AddYak` use case replay) that
   are not exercised during normal command processing. This
   means the "rebuild from events" path gets less testing than
   the "load from projection" path.

5. **Event replay on the aggregate is not possible.** Since
   `YakMap` has no `apply(event)` method, you cannot replay a
   sequence of events to reach a historical state. Any future
   feature requiring aggregate-level temporal queries (e.g.,
   "what was the yak map state at event #42?") would need a new
   mechanism.

## Consequences

### What this enables

- Keeping the aggregate simple and focused on business rules
- Avoiding the complexity of event upcasting during aggregate
  load
- O(current-state) load time rather than O(event-history) load
  time
- A clean separation where the aggregate does not depend on the
  event store at all

### What this constrains

- The projection must be correct for the aggregate to function
  correctly — there is no independent verification
- Drift between the event store and projection is undetectable
  without manual inspection
- Adding an `apply(event)` method to `YakMap` later would be a
  significant refactor, touching all event types

### When this might need to change

- **If projection bugs cause data corruption in practice.** A
  sequence-number check (aggregate loads from projection,
  verifies the event store has no events beyond the projection's
  recorded sequence) would detect drift without full event
  replay.
- **If async or remote projections are introduced.** The current
  trade-off relies on synchronous, co-located projection updates.
  Eventual consistency would widen the drift window
  unacceptably.
- **If temporal queries on the aggregate are needed.** This
  would require either event replay capability in `YakMap` or a
  separate mechanism to reconstruct historical aggregate state.
- **If the event schema changes frequently.** Currently the
  aggregate avoids upcasting because it never reads events. If
  events need to be replayed for any reason, schema evolution
  (ADR 0011) becomes relevant to aggregate loading.
