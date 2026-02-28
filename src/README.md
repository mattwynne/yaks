# Architecture

Hexagonal architecture (ports & adapters) with CQRS and Event
Sourcing. See [ADR 0001](../docs/adr/0001-migrate-from-bash-to-rust-for-core-implementation.md)
and [ADR 0002](../docs/adr/0002-adopt-cqrs-and-event-sourcing.md) for
the decisions behind this.

## How a Command Flows

```
CLI (main.rs)
  -> Application use case (e.g. AddYak)
    -> YakMap aggregate (domain logic, emits YakEvent)
    -> EventStore.append (persists event to git refs)
    -> EventBus.notify (fans out to listeners)
      -> WriteYakStore projection (updates .yaks/ directories)
```

`main.rs` is deliberately thin: it parses CLI args (clap), wires
adapters to ports, and routes commands to use cases. See
[ADR 0008](../docs/adr/0008-keep-main-rs-thin-only-wiring-and-routing.md).

## Layers

### Domain (`domain/`)

Core business logic. No dependencies on infrastructure.

- **`yak.rs`** - `YakView` read-model DTO (id, name, parent_id, state, context, fields, children)
- **`yak_map.rs`** - `YakMap` aggregate: the write model that enforces all
  invariants (e.g. parent must exist, parent can't be done if children aren't)
- **`event.rs`** - `YakEvent` enum: Added, Removed, Moved, FieldUpdated.
  Events carry `EventMetadata` (author, timestamp).
- **`slug.rs`** - Identity types: `YakId` (immutable), `Slug` (filesystem),
  `Name` (display). See [ADR 0005](../docs/adr/0005-identity-model-for-yaks.md).
- **`field.rs`** - Field name validation, reserved field constants
  (state, name, context, metadata, id)
- **`ports/`** - Trait definitions for all external dependencies:
  - `EventStore` / `EventStoreReader` - event persistence and sync
  - `ReadYakStore` / `WriteYakStore` - yak storage (read model)
  - `DisplayPort` - user output
  - `InputPort` - user input (stdin/editor)
  - `AuthenticationPort` - author identity
  - `EventListener` - reacts to domain events

### Application (`application/`)

Use cases that orchestrate domain + ports. Each use case is a struct
with an `execute` method.

- `Application` (`app.rs`) - holds references to all ports, dispatches
  commands to the `YakMap` aggregate, appends events, notifies the bus
- Use cases: `AddYak`, `ListYaks`, `DoneYak`, `StartYak`, `RemoveYak`,
  `PruneYaks`, `EditContext`, `ShowContext`, `SetState`, `MoveYak`,
  `RenameYak`, `ShowField`, `WriteField`, `ShowLog`, `SyncYaks`,
  `Completions`

### Adapters (`adapters/`)

Implementations of port traits for specific technologies.

- **`event_store/`** - `GitEventStore`: persists events as git tree
  entries under `refs/notes/yaks`. Handles sync via fetch/push with
  CRDT-style conflict resolution. See
  [ADR 0007](../docs/adr/0007-sync-is-an-eventstore-responsibility-not-a-separate-port.md).
- **`yak_store/`** - `DirectoryStorage`: file-based read model under
  `$YAK_PATH` (defaults to `.yaks/`). Each yak is a directory with
  field files (state, context.md, etc).
- **`user_display/`** - `ConsoleDisplay`: terminal output with colours
- **`user_input/`** - `ConsoleInput`: stdin and $EDITOR input
- **`authentication/`** - `GitAuthentication`: reads author from git config

Each adapter module also provides an `InMemory*` variant for testing.

### Infrastructure (`infrastructure/`)

Cross-cutting concerns that aren't domain or adapter logic.

- **`event_bus.rs`** - `EventBus`: fans out domain events to registered
  `EventListener` implementations. Supports `rebuild` for full replay.
- **`git_discovery.rs`** - finds the git repo root and validates the
  working environment

### Projections (`projections/`)

CQRS read model projections. These subscribe to events and maintain
queryable views.

- **`write_to_yak_store.rs`** - bridges `EventListener` to
  `WriteYakStore`: any `WriteYakStore` implementation automatically
  becomes an `EventListener` that keeps the read model in sync with
  domain events.
