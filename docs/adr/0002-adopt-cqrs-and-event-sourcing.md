# 2. Adopt CQRS and Event Sourcing

Date: 2026-02-12

## Status

accepted

## Context

Following the migration from Bash to Rust (ADR 0001), we had a
clean hexagonal architecture with ports and adapters. The core
storage model was directory-based: each yak is a directory under
`.yaks/` with `state` and `context.md` files. This worked well
for simple operations but had limitations:

**Audit trail**: No record of what happened or when. If a yak's
state changed, the previous state was lost. Understanding the
history of a project required external tooling (git log on the
`.yaks/` directory).

**Collaboration model**: The directory-based model stores current
state only. Synchronising state between collaborators (multiple
developers or agents working on yaks simultaneously) requires
diffing and merging directory trees, which is brittle and lossy.

**Domain complexity**: Yak hierarchies have non-trivial business
rules. When a child yak is marked "wip", its ancestors should
propagate to "wip". When a parent is marked "done", all children
must already be "done". These rules involve reading and writing
multiple yaks atomically, which is awkward with per-file storage.

**Future sync vision**: We want yaks to be shareable across git
branches and repositories using git refs. An event-based model
maps naturally to git's append-only commit model, where each
event becomes a commit on a refs/notes branch.

## Decision

Adopt CQRS (Command Query Responsibility Segregation) with Event
Sourcing as the core architectural pattern.

### Command side

All mutations flow through the `YakMap` aggregate, which:

1. Loads current state from the read model (`ReadYakStore`)
2. Validates business rules (name validation, hierarchy rules,
   state transition constraints)
3. Emits domain events (`YakEvent` variants) without side effects
4. Returns pending events to the caller

Events are the source of truth. The six domain events are:

- `Added` - yak created
- `Removed` - yak deleted
- `Moved` - yak renamed or relocated in hierarchy
- `ContextUpdated` - context field changed
- `StateUpdated` - state field changed (todo/wip/done)
- `FieldUpdated` - custom field changed

Use cases implement the `UseCase` trait and call
`Application::with_yak_map()` to execute domain logic. The
`Application` collects events from the aggregate and publishes
them through the `EventBus`.

### Event infrastructure

The `EventBus` guarantees ordering:

1. Persist event to `EventStore` (append-only log)
2. Notify all registered `EventListener` projections

This dual-write ensures events are durably stored before any
projection updates, so projections can always be rebuilt from
the event store.

Two `EventStore` implementations exist:

- `InMemoryEventStore` for tests
- `GitEventStore` backed by `refs/notes/yaks` for production

### Query side

The read model is the existing directory-based storage, now
treated as a projection rather than the source of truth. The
`WriteToYakStore` projection listens for events and updates
the filesystem accordingly. Any `WriteYakStore` implementation
automatically becomes an `EventListener` via a blanket impl.

Queries read from `ReadYakStore` (the projected state) and
never go through the aggregate. The `Application` struct holds
separate references to the `EventBus` (write) and
`ReadYakStore` (read), making the separation explicit.

### Current architecture diagram

```
  Command flow:
  CLI -> UseCase -> Application.with_yak_map() -> YakMap
    -> [domain events] -> EventBus
      -> EventStore (persist)
      -> WriteToYakStore projection (update filesystem)

  Query flow:
  CLI -> UseCase -> ReadYakStore (filesystem read)
```

## Consequences

### What becomes easier

**Audit trail**: Every mutation is recorded as an event. The
`yx log` command shows the full history of operations. Events
are human-readable (`Added: "my yak"`, `StateUpdated: "my yak"
"wip"`).

**Git-native sync**: Events map directly to git commits on a
refs/notes branch. The `GitEventStore` adapter already
implements this. Syncing between collaborators becomes merging
event logs rather than diffing directory trees.

**Testability**: The aggregate (`YakMap`) is a pure in-memory
data structure. Business rules can be tested by feeding events
and asserting emitted events, without touching the filesystem
or any I/O.

**Complex business rules**: Atomic operations across multiple
yaks (hierarchy propagation, cascade completion checks) happen
inside the aggregate before any events are emitted. No partial
updates are possible.

**Rebuildable state**: The read model can be fully reconstructed
by replaying the event log. If the directory-based storage gets
corrupted, it can be rebuilt from events.

**Future projections**: New read models can be added by
registering additional `EventListener` implementations. For
example, a search index, a dashboard view, or a different
storage backend could all consume the same event stream.

### What becomes harder

**Conceptual overhead**: CQRS/ES is a more complex pattern than
simple CRUD. Contributors need to understand the event flow,
aggregate boundaries, and projection model.

**Eventual consistency risk**: The read model lags behind events
by the time it takes for the projection to run. In practice this
is synchronous and immediate today, but the architecture admits
the possibility of async projections in future.

**Event schema evolution**: Once events are persisted, their
format is a contract. Changing event structure requires
migration or upcasting strategies. No versioning mechanism
exists yet.

**Aggregate loading cost**: The aggregate is currently rebuilt
from the read model on every command (`YakMap::from_store()`).
This is effectively loading from a snapshot (the projected
filesystem state), so performance scales with the number of
yaks, not the number of events. For very large yak collections
the filesystem I/O could become slow, but this is unlikely to
be a practical concern.

**Debugging**: Understanding system state requires tracing the
flow from command through aggregate through event bus through
projection. More moving parts than direct file writes.

### Open questions for future work

- **Event versioning**: How to handle breaking changes to event
  schemas as the domain model evolves.
- **Snapshot version linkage**: The directory-based read model
  already functions as an informal snapshot — the aggregate
  loads from it rather than replaying events. However, it has
  no version link to the event stream (no "as of event #N").
  If the read model and event store ever drift apart (crash,
  bug, async projections), there is no mechanism to detect or
  recover from the inconsistency. Adding a sequence number
  would make this a formal snapshot.
- **Async projections**: Whether projections should remain
  synchronous or move to eventual consistency.
- **Additional read models**: What specialised projections
  would be valuable beyond filesystem storage.
