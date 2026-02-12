# Unified Git Event Store

## Summary

Replace the `InMemoryEventStore` with a `GitEventStore` that
uses git's object database as the append-only event log and
source of truth. Remove the separate `LogPort`/`GitLog`
abstraction, which becomes redundant.

## What Already Exists

The codebase already has an event-sourced architecture:

- **EventStore port** (`src/ports/event_store.rs`): trait with
  `append`, `get_events`, `get_all_events`
- **EventListener port** (`src/ports/event_listener.rs`):
  trait with `on_event`
- **EventBus** (`src/infrastructure/event_bus.rs`): publishes
  to EventStore + notifies EventListeners
- **DirectoryStorage** already implements EventListener,
  updating `.yaks/` from events
- **Use cases** already publish events and read from Store

What's missing is a **git-backed EventStore**. Currently
`InMemoryEventStore` is used in production, and `GitLog`
(implementing `LogPort`) separately logs commands to git.
These two concerns should be one thing.

## Motivation

- **Simplicity**: One persistence abstraction instead of two
- **Event sourcing**: Git is the single source of truth;
  state can be rebuilt by replaying events
- **Richer history**: Full event data stored as git objects,
  not just command strings
- **Future sync**: Foundation for multi-branch collaboration
  via git refs

## Architecture

The architecture stays the same; only the EventStore
implementation changes:

```
EventBus.publish(event)
  ├→ EventStore.append(event)       [GitEventStore → .git/objects/]
  └→ EventListeners.on_event(event) [DirectoryStorage → .yaks/]
```

### Key principle: no shared writes

GitEventStore and DirectoryStorage are independent projections
of the same events. GitEventStore builds git trees purely in
`.git/objects/` using git plumbing (no filesystem writes to
`.yaks/`). DirectoryStorage updates `.yaks/` from event data.
They do not read each other's output.

Both run synchronously within `EventBus.publish()`, but order
does not matter since neither depends on the other.

### Read path (unchanged)

Use cases read from Store (DirectoryStorage), which reads
`.yaks/`. No read-after-write issue because use cases read
before mutating, then publish events at the end.

## EventStore Port

```rust
pub trait EventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()>;
    fn get_events(&self, name: &str) -> Result<Vec<YakEvent>>;
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
}
```

- `append`: Writes event to store
- `get_events`: Query by yak name (git-native `--grep` filtering)
- `get_all_events`: Walk entire log

## Git Commit Format

Each event is a commit on `refs/notes/yaks`:

- **Commit message**: Human-readable tagged format
- **Tree**: Full `.yaks/` state built from event data
- **Git metadata**: Timestamp, author (provided by git)

### Tagged message format

```
Added: "foo"
Removed: "foo"
Moved: "old-name" "new-name"
ContextUpdated: "foo"
StateUpdated: "foo" "wip"
FieldUpdated: "foo" "parent"
```

Multi-value fields use quoted values. Quotes are forbidden in
yak names by `validate_yak_name`, so no escaping is needed.

Content like context text is not duplicated in the message
because it is already present in the committed tree. When
reading events back, content fields are left empty. If full
content is needed in future, it can be recovered by diffing
the commit's tree against its parent.

## GitEventStore Adapter

Implements `EventStore`. Uses git2 to build trees directly
in `.git/objects/` without touching the filesystem.

See: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

### Writing (append)

1. Get current tree from latest commit on `refs/notes/yaks`
   (if exists)
2. Apply event's changes to tree structure:
   - **Added**: Create `foo/context.md` (empty blob),
     `foo/state` (blob containing "todo")
   - **ContextUpdated**: Update `foo/context.md` blob
   - **StateUpdated**: Update `foo/state` blob
   - **Removed**: Remove `foo/` subtree
   - **Moved**: Rename subtree
   - **FieldUpdated**: Update `foo/<field_name>` blob
3. Create blobs and tree objects via git2
4. Commit with tagged message to `refs/notes/yaks`

### Reading (get_events, get_all_events)

1. Walk commits on `refs/notes/yaks` using `revwalk`
2. Parse commit messages back into `YakEvent` variants
3. For `get_events(name)`: use git-native grep filtering
4. Return events in chronological order

## Trait-Based Event Types

Each event variant becomes a separate struct implementing
`EventFormat`. The `YakEvent` enum wraps and delegates.

```rust
pub trait EventFormat {
    fn event_tag(&self) -> &'static str;
    fn format_data(&self) -> String;
    fn parse_data(data: &str) -> Result<Self> where Self: Sized;
}

pub struct AddedEvent { pub name: String }
impl EventFormat for AddedEvent {
    fn event_tag(&self) -> &'static str { "Added" }
    fn format_data(&self) -> String {
        format!("\"{}\"", self.name)
    }
    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        Ok(Self { name: values[0].clone() })
    }
}

/// Parse space-separated quoted values: "foo" "bar" → ["foo", "bar"]
fn parse_quoted_values(data: &str) -> Result<Vec<String>>

pub struct StateUpdatedEvent {
    pub name: String,
    pub state: String,
}
impl EventFormat for StateUpdatedEvent {
    fn event_tag(&self) -> &'static str { "StateUpdated" }
    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.name, self.state)
    }
    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        Ok(Self {
            name: values[0].clone(),
            state: values[1].clone(),
        })
    }
}

// Similarly: RemovedEvent, MovedEvent, ContextUpdatedEvent,
//            FieldUpdatedEvent

pub enum YakEvent {
    Added(AddedEvent),
    Removed(RemovedEvent),
    Moved(MovedEvent),
    ContextUpdated(ContextUpdatedEvent),
    StateUpdated(StateUpdatedEvent),
    FieldUpdated(FieldUpdatedEvent),
}

impl YakEvent {
    pub fn format_message(&self) -> String {
        // Delegates to inner type's event_tag + format_data
    }

    pub fn parse(message: &str) -> Result<Self> {
        // Split on ": ", match tag, delegate to parse_data
    }
}
```

Adding a new event type requires:

1. Create the struct
2. Implement `EventFormat`
3. Add variant to `YakEvent`
4. Add one line to each match in `format_message` / `parse`

## DirectoryStorage Snapshot Initialization

DirectoryStorage already implements EventListener. The new
addition is snapshot initialization: on startup, it can
restore from the latest git tree instead of requiring events
to be replayed.

```rust
impl DirectoryStorage {
    pub fn new_from_snapshot(
        yak_path: &Path,
        repo: &Repository,
    ) -> Result<Self> {
        // Read latest tree from refs/notes/yaks
        // Materialize to .yaks/ directory
    }

    pub fn new_empty(yak_path: &Path) -> Self {
        // Fresh start, no snapshot
    }
}
```

This enables rebuilding `.yaks/` from git if the directory
is missing or stale.

## What Gets Removed

- `LogPort` trait
- `GitLog` adapter
- Legacy `Event` struct (operation/args/stdin/timestamp/author)

## What Gets Added

- `GitEventStore` adapter (implements existing `EventStore` trait)
- `EventFormat` trait + individual event structs
- Snapshot initialization for DirectoryStorage
- Contract tests (macro-based)

## What Changes

- `InMemoryEventStore` replaced by `GitEventStore` in production
  wiring (`main.rs`); `InMemoryEventStore` kept for tests
- `yx log` reimplemented to read from `EventStore.get_all_events()`
  instead of `GitLog.read_events()`. Output format changes from
  `{timestamp} {author} {operation} {args}` to
  `{timestamp} {author} {event_tag}: {data}`.
  This is acceptable since log format was not a stable API.

## Testing Strategy

### Contract tests (macro-based)

Both `InMemoryEventStore` and `GitEventStore` must pass
identical contract tests:

```rust
macro_rules! event_store_tests {
    ($create_store:expr) => {
        #[test]
        fn appends_and_retrieves_event() {
            let mut store = $create_store;
            // ...
        }

        #[test]
        fn filters_events_by_name() {
            let mut store = $create_store;
            // ...
        }

        #[test]
        fn roundtrips_each_event_type() {
            // ...
        }
    }
}

mod in_memory_event_store {
    use super::*;
    event_store_tests!(InMemoryEventStore::new());
}

mod git_event_store {
    use super::*;
    event_store_tests!({
        let tmp = tempdir().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        GitEventStore::new(&repo)
    });
}
```

### Unit tests

- `EventFormat` roundtrip per event type (serialize/parse)
- GitEventStore tree building

### Integration tests

- DirectoryStorage as EventListener (event in, check filesystem)

### Acceptance tests

- ShellSpec tests unchanged (test CLI, not internals)
- `yx log` ShellSpec tests updated for new output format
