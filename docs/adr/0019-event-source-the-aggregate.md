# 19. Event-source the Aggregate

Date: 2026-03-07

## Status

accepted (supersedes ADR 0015)

## Context

ADR 0015 documented that the aggregate loaded from the projected
read model (the `.yaks/` filesystem) rather than by replaying
events from the event store. This meant the system was not purely
event-sourced on the command side — the aggregate trusted a
derived view rather than the authoritative event log.

That approach had several significant risks:

1. **Projection bugs corrupted the aggregate.** If the projection
   wrote incorrect state, the aggregate loaded that corrupt state
   and made decisions based on faulty premises. The event log
   remained technically correct, but the decisions were wrong.

2. **No version linkage.** The projection had no marker
   indicating which events it reflected. Drift between the
   projection and event store was undetectable.

3. **The aggregate couldn't detect missed events.** Since
   `from_store()` didn't consult the event store, it had no way
   to know if the projection was behind, ahead, or diverged.

4. **Event replay was a separate code path.** Commands like
   `yx reset` and `yx compact` reconstructed state from events,
   but used different mechanisms that weren't exercised during
   normal command processing. This meant the "rebuild from
   events" path got less testing than the "load from projection"
   path.

5. **No `apply(event)` method.** The aggregate had no mechanism
   to apply events to its state, making it impossible to replay
   events or reach historical states.

Since ADR 0015, the system has evolved:

- **ADR 0018 (Unify Yak domain types)** consolidated multiple
  overlapping types (YakView, YakEntry, YakSnapshot) into a
  single `Yak` struct, simplifying the domain model.

- The `YakMap` aggregate now has a full `apply(event)` method
  that handles all event types: Added, Removed, Moved,
  FieldUpdated, and Compacted.

- The `from_events()` method was implemented to reconstruct
  aggregate state by replaying events from the event store.

## Decision

Make the `YakMap` aggregate truly event-sourced by loading it
from events rather than from the projected read model.

### Implementation

1. **`from_events()` is the primary loading path.**
   `Application::with_yak_map()` calls `YakMap::from_events()`
   to reconstruct aggregate state by replaying events from the
   `EventStore`.

2. **`apply(event)` handles all event types.** The aggregate has
   a complete event application method that updates state for:
   - `Added`: creates new yak with initial state
   - `Removed`: removes yak from the map
   - `Moved`: updates parent_id
   - `FieldUpdated`: handles .state, .context.md, .name, .tags,
     and custom fields
   - `Compacted`: replaces the entire aggregate state

3. **`from_store()` retained for backward compatibility.** The
   old projection-based loading method still exists but is used
   only as a fallback. New code paths use `from_events()`.

4. **The read model is purely a projection.** The `.yaks/`
   directory is now exclusively a query-side concern. It's not
   involved in command-side state reconstruction.

### Relation to other ADRs

- **Supersedes ADR 0015.** The aggregate no longer loads from
  the projected snapshot. It loads from events.

- **Builds on ADR 0002 (CQRS and Event Sourcing).** The command
  side is now truly event-sourced, not just event-logged.

- **Depends on ADR 0018 (Unify Yak types).** The single `Yak`
  type simplifies event application — no need to convert between
  YakView, YakEntry, and YakSnapshot.

## Consequences

### Benefits

- **The aggregate is derived from the authoritative source.**
  State is built from the event log, not from a derived
  projection. This eliminates the risk of projection bugs
  corrupting command-side decisions.

- **Single code path for state reconstruction.** The `apply()`
  method is exercised on every command, not just during reset or
  compact. This means the event replay path gets continuous
  testing.

- **No drift between projection and aggregate.** The projection
  can be regenerated from events at any time. The aggregate
  doesn't depend on it.

- **Temporal queries become possible.** With `from_events()` and
  `apply()`, you can replay events up to any point to see
  historical aggregate state (though this is not currently
  exposed in the CLI).

### Trade-offs

- **Performance shift.** Loading now scales with
  O(events-since-last-compaction) instead of
  O(current-state-size). However, compaction keeps the event
  count bounded, so this remains manageable.

- **More code to maintain.** The `apply()` method must handle
  all event types correctly. Schema evolution (ADR 0011) now
  affects aggregate loading, not just the event store.

### Future considerations

- **Compaction becomes more important.** Since every command
  replays all events since the last compaction, compaction
  frequency affects command latency. Monitor replay time and
  compact proactively if needed.

- **Event upcasting during replay.** If event schemas change
  (ADR 0011), the aggregate's `apply()` method may need to
  handle older event versions. This is now a concern where it
  wasn't before.

- **Snapshot optimization.** If event replay becomes a
  performance bottleneck, we could introduce true snapshots
  (events marked with sequence numbers) to avoid replaying the
  full history. But the current approach is simpler and
  sufficient.
