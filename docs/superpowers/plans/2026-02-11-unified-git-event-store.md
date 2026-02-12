# Unified Git Event Store Implementation Plan

> **For Claude:** REQUIRED: Use superpowers:subagent-driven-development
> (if subagents available) or superpowers:executing-plans to implement
> this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `InMemoryEventStore` + `LogPort`/`GitLog` with a
single `GitEventStore` that uses git's object database as the
append-only event log.

**Architecture:** The existing EventBus/EventStore/EventListener
architecture stays the same. We're adding a `GitEventStore` adapter
that implements the existing `EventStore` trait, building git trees
from event data using git2 plumbing. We also refactor `YakEvent` to
use individual event structs with an `EventFormat` trait for
serialization.

**Tech Stack:** Rust, git2 crate (already a dependency)

**Spec:** `docs/superpowers/specs/2026-02-11-unified-git-event-store-design.md`

**Working directory:** `/Users/mattwynne/git/mattwynne/yaks/.worktrees/unified-git-event-store`

**Commands:**
- Run tests: `cargo test`
- Run specific test: `cargo test test_name`
- Run lint: `cargo clippy -- -D warnings && cargo fmt -- --check`
- Run all checks from worktree: `cd /Users/mattwynne/git/mattwynne/yaks && dev check`
- Build release binary: `cargo build --release` (needed before ShellSpec tests)
- Commit: `git mit me && git add <files> && git commit -m "message"`

---

## Chunk 1: EventFormat Trait + YakEvent Refactoring

This chunk restructures `YakEvent` from an enum with inline fields to
an enum wrapping individual structs, each implementing an `EventFormat`
trait for serialization/deserialization.

### File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/domain/event_format.rs` | Create | `EventFormat` trait + `parse_quoted_values` helper |
| `src/domain/events/mod.rs` | Create | Module for individual event structs |
| `src/domain/events/added.rs` | Create | `AddedEvent` struct + `EventFormat` impl |
| `src/domain/events/removed.rs` | Create | `RemovedEvent` struct + `EventFormat` impl |
| `src/domain/events/moved.rs` | Create | `MovedEvent` struct + `EventFormat` impl |
| `src/domain/events/context_updated.rs` | Create | `ContextUpdatedEvent` struct + `EventFormat` impl |
| `src/domain/events/state_updated.rs` | Create | `StateUpdatedEvent` struct + `EventFormat` impl |
| `src/domain/events/field_updated.rs` | Create | `FieldUpdatedEvent` struct + `EventFormat` impl |
| `src/domain/event.rs` | Modify | Refactor `YakEvent` to wrap structs, add `format_message`/`parse` |
| `src/domain/mod.rs` | Modify | Add `event_format` and `events` modules |

After this chunk, all existing code still works - the enum variants
change shape but the compiler guides every update.

---

### Task 1: Create EventFormat trait and parse_quoted_values

**Files:**
- Create: `src/domain/event_format.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Write EventFormat trait and parse_quoted_values with tests**

Create `src/domain/event_format.rs`:

```rust
use anyhow::Result;

/// Trait for serializing/deserializing individual event types
pub trait EventFormat {
    /// Tag name for this event (e.g., "Added", "StateUpdated")
    fn event_tag(&self) -> &'static str;
    /// Serialize event data (everything after "Tag: ")
    fn format_data(&self) -> String;
    /// Deserialize event data from string
    fn parse_data(data: &str) -> Result<Self>
    where
        Self: Sized;
}

/// Parse space-separated quoted values: `"foo" "bar"` → `["foo", "bar"]`
pub fn parse_quoted_values(data: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    let mut chars = data.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek() == Some(&' ') {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        // Expect opening quote
        if chars.next() != Some('"') {
            anyhow::bail!("Expected '\"' in event data: {}", data);
        }
        // Read until closing quote
        let mut value = String::new();
        loop {
            match chars.next() {
                Some('"') => break,
                Some(c) => value.push(c),
                None => anyhow::bail!("Unterminated quote in event data: {}", data),
            }
        }
        values.push(value);
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_quoted_value() {
        let values = parse_quoted_values("\"foo\"").unwrap();
        assert_eq!(values, vec!["foo"]);
    }

    #[test]
    fn parses_multiple_quoted_values() {
        let values = parse_quoted_values("\"foo\" \"bar\"").unwrap();
        assert_eq!(values, vec!["foo", "bar"]);
    }

    #[test]
    fn parses_values_with_spaces() {
        let values = parse_quoted_values("\"foo bar\" \"baz\"").unwrap();
        assert_eq!(values, vec!["foo bar", "baz"]);
    }

    #[test]
    fn errors_on_missing_quote() {
        assert!(parse_quoted_values("foo").is_err());
    }

    #[test]
    fn errors_on_unterminated_quote() {
        assert!(parse_quoted_values("\"foo").is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test parse_quoted`
Expected: 5 tests pass

- [ ] **Step 3: Register module in domain/mod.rs**

In `src/domain/mod.rs`, add after `pub mod yak_map;`:
```rust
pub mod event_format;
```

And add to the `pub use` block (with `#[allow(unused_imports)]`
matching existing pattern):
```rust
#[allow(unused_imports)]
pub use event_format::{parse_quoted_values, EventFormat};
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: All 143 tests pass (no regressions)

- [ ] **Step 5: Commit**

```bash
git mit me && git add src/domain/event_format.rs src/domain/mod.rs
git commit -m "Add EventFormat trait and quoted value parser"
```

---

### Task 2: Create individual event structs with EventFormat

**Files:**
- Create: `src/domain/events/added.rs`
- Create: `src/domain/events/removed.rs`
- Create: `src/domain/events/moved.rs`
- Create: `src/domain/events/context_updated.rs`
- Create: `src/domain/events/state_updated.rs`
- Create: `src/domain/events/field_updated.rs`
- Create: `src/domain/events/mod.rs`
- Modify: `src/domain/mod.rs`

Each struct mirrors one YakEvent variant. Each implements EventFormat
with roundtrip tests.

- [ ] **Step 1: Create AddedEvent**

Create `src/domain/events/added.rs`:
```rust
use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedEvent {
    pub name: String,
}

impl EventFormat for AddedEvent {
    fn event_tag(&self) -> &'static str {
        "Added"
    }

    fn format_data(&self) -> String {
        format!("\"{}\"", self.name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Added event requires a name");
        Ok(Self {
            name: values[0].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = AddedEvent {
            name: "test yak".to_string(),
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn event_tag() {
        let event = AddedEvent {
            name: "test".to_string(),
        };
        assert_eq!(event.event_tag(), "Added");
    }
}
```

- [ ] **Step 2: Create RemovedEvent**

Create `src/domain/events/removed.rs`:
```rust
use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedEvent {
    pub name: String,
}

impl EventFormat for RemovedEvent {
    fn event_tag(&self) -> &'static str {
        "Removed"
    }

    fn format_data(&self) -> String {
        format!("\"{}\"", self.name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Removed event requires a name");
        Ok(Self {
            name: values[0].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = RemovedEvent {
            name: "test yak".to_string(),
        };
        let data = event.format_data();
        let parsed = RemovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
```

- [ ] **Step 3: Create MovedEvent**

Create `src/domain/events/moved.rs`:
```rust
use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedEvent {
    pub old_name: String,
    pub new_name: String,
}

impl EventFormat for MovedEvent {
    fn event_tag(&self) -> &'static str {
        "Moved"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.old_name, self.new_name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(values.len() >= 2, "Moved event requires old and new names");
        Ok(Self {
            old_name: values[0].clone(),
            new_name: values[1].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = MovedEvent {
            old_name: "old name".to_string(),
            new_name: "new name".to_string(),
        };
        let data = event.format_data();
        let parsed = MovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
```

- [ ] **Step 4: Create ContextUpdatedEvent**

Create `src/domain/events/context_updated.rs`:
```rust
use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

/// Note: `content` is NOT serialized in the commit message because it
/// is stored in the git tree (context.md blob). When reading events
/// back from git, `content` will be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextUpdatedEvent {
    pub name: String,
    pub content: String,
}

impl EventFormat for ContextUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "ContextUpdated"
    }

    fn format_data(&self) -> String {
        format!("\"{}\"", self.name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "ContextUpdated event requires a name");
        Ok(Self {
            name: values[0].clone(),
            content: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_excludes_content() {
        let event = ContextUpdatedEvent {
            name: "test yak".to_string(),
            content: "some long context".to_string(),
        };
        assert_eq!(event.format_data(), "\"test yak\"");
    }

    #[test]
    fn parse_sets_empty_content() {
        let parsed = ContextUpdatedEvent::parse_data("\"test yak\"").unwrap();
        assert_eq!(parsed.name, "test yak");
        assert_eq!(parsed.content, "");
    }
}
```

- [ ] **Step 5: Create StateUpdatedEvent**

Create `src/domain/events/state_updated.rs`:
```rust
use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateUpdatedEvent {
    pub name: String,
    pub state: String,
}

impl EventFormat for StateUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "StateUpdated"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.name, self.state)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(values.len() >= 2, "StateUpdated event requires name and state");
        Ok(Self {
            name: values[0].clone(),
            state: values[1].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = StateUpdatedEvent {
            name: "test yak".to_string(),
            state: "wip".to_string(),
        };
        let data = event.format_data();
        let parsed = StateUpdatedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
```

- [ ] **Step 6: Create FieldUpdatedEvent**

Create `src/domain/events/field_updated.rs`:
```rust
use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

/// Note: `content` is NOT serialized in the commit message because it
/// is stored in the git tree (as a blob). When reading events back
/// from git, `content` will be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldUpdatedEvent {
    pub name: String,
    pub field_name: String,
    pub content: String,
}

impl EventFormat for FieldUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "FieldUpdated"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.name, self.field_name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(values.len() >= 2, "FieldUpdated event requires name and field_name");
        Ok(Self {
            name: values[0].clone(),
            field_name: values[1].clone(),
            content: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_excludes_content() {
        let event = FieldUpdatedEvent {
            name: "test yak".to_string(),
            field_name: "notes".to_string(),
            content: "stuff".to_string(),
        };
        assert_eq!(event.format_data(), "\"test yak\" \"notes\"");
    }

    #[test]
    fn parse_sets_empty_content() {
        let parsed = FieldUpdatedEvent::parse_data("\"test yak\" \"notes\"").unwrap();
        assert_eq!(parsed.name, "test yak");
        assert_eq!(parsed.field_name, "notes");
        assert_eq!(parsed.content, "");
    }
}
```

- [ ] **Step 7: Create events/mod.rs and register in domain/mod.rs**

Create `src/domain/events/mod.rs` (now that all files exist):
```rust
pub mod added;
pub mod context_updated;
pub mod field_updated;
pub mod moved;
pub mod removed;
pub mod state_updated;

pub use added::AddedEvent;
pub use context_updated::ContextUpdatedEvent;
pub use field_updated::FieldUpdatedEvent;
pub use moved::MovedEvent;
pub use removed::RemovedEvent;
pub use state_updated::StateUpdatedEvent;
```

In `src/domain/mod.rs`, add after `pub mod event_format;`:
```rust
pub mod events;
```

And add to the `pub use` block:
```rust
#[allow(unused_imports)]
pub use events::{
    AddedEvent, ContextUpdatedEvent, FieldUpdatedEvent,
    MovedEvent, RemovedEvent, StateUpdatedEvent,
};
```

- [ ] **Step 8: Run tests**

Run: `cargo test`
Expected: All existing 143 tests pass + new roundtrip tests pass

- [ ] **Step 9: Commit**

```bash
git mit me && git add src/domain/events/ src/domain/mod.rs
git commit -m "Add individual event structs with EventFormat"
```

---

### Task 3: Refactor YakEvent to wrap event structs

This is a mechanical refactoring. Change the YakEvent enum variants
from inline fields to wrapping the new structs, then fix every compile
error. The Rust compiler will find every site that needs updating.

**Files to modify** (compiler will guide you to all of these):
- `src/domain/event.rs` - enum definition + tests
- `src/domain/yak.rs` - event creation + tests
- `src/domain/yak_map.rs` - event creation + tests
- `src/application/prune_yaks.rs` - event creation
- `src/adapters/storage/directory.rs` - EventListener match + tests
- `src/adapters/storage/memory.rs` - EventListener match + tests
- `src/adapters/event_store/memory.rs` - get_events filter
- `src/adapters/log/git_log.rs` - EventListener match
- `src/ports/event_store.rs` - test inline EventStore
- `src/ports/event_listener.rs` - test
- `src/infrastructure/event_bus.rs` - test

- [ ] **Step 1: Change YakEvent enum definition**

In `src/domain/event.rs`, replace the entire file with:
```rust
// Event domain model - represents a logged yak operation

use anyhow::Result;

use super::event_format::EventFormat;
use super::events::*;

#[derive(Debug, Clone, PartialEq, Eq)]
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
        match self {
            Self::Added(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Removed(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Moved(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::ContextUpdated(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::StateUpdated(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::FieldUpdated(e) => format!("{}: {}", e.event_tag(), e.format_data()),
        }
    }

    pub fn parse(message: &str) -> Result<Self> {
        let (tag, data) = message
            .split_once(": ")
            .ok_or_else(|| anyhow::anyhow!("Invalid event format: {}", message))?;
        match tag {
            "Added" => Ok(Self::Added(AddedEvent::parse_data(data)?)),
            "Removed" => Ok(Self::Removed(RemovedEvent::parse_data(data)?)),
            "Moved" => Ok(Self::Moved(MovedEvent::parse_data(data)?)),
            "ContextUpdated" => Ok(Self::ContextUpdated(ContextUpdatedEvent::parse_data(data)?)),
            "StateUpdated" => Ok(Self::StateUpdated(StateUpdatedEvent::parse_data(data)?)),
            "FieldUpdated" => Ok(Self::FieldUpdated(FieldUpdatedEvent::parse_data(data)?)),
            _ => anyhow::bail!("Unknown event type: {}", tag),
        }
    }

    /// Get the yak name this event affects (for filtering)
    pub fn yak_name(&self) -> &str {
        match self {
            Self::Added(e) => &e.name,
            Self::Removed(e) => &e.name,
            Self::Moved(e) => &e.old_name,
            Self::ContextUpdated(e) => &e.name,
            Self::StateUpdated(e) => &e.name,
            Self::FieldUpdated(e) => &e.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_message_added() {
        let event = YakEvent::Added(AddedEvent {
            name: "test yak".to_string(),
        });
        assert_eq!(event.format_message(), "Added: \"test yak\"");
    }

    #[test]
    fn format_message_state_updated() {
        let event = YakEvent::StateUpdated(StateUpdatedEvent {
            name: "test".to_string(),
            state: "wip".to_string(),
        });
        assert_eq!(event.format_message(), "StateUpdated: \"test\" \"wip\"");
    }

    #[test]
    fn parse_roundtrip() {
        let event = YakEvent::Added(AddedEvent {
            name: "test".to_string(),
        });
        let msg = event.format_message();
        let parsed = YakEvent::parse(&msg).unwrap();
        assert_eq!(parsed, event);
    }

    #[test]
    fn parse_unknown_tag_errors() {
        assert!(YakEvent::parse("Unknown: \"foo\"").is_err());
    }

    #[test]
    fn yak_name_returns_correct_name() {
        let event = YakEvent::Moved(MovedEvent {
            old_name: "old".to_string(),
            new_name: "new".to_string(),
        });
        assert_eq!(event.yak_name(), "old");
    }
}
```

This removes the legacy `Event` struct, `chrono` import, and
`serde` derives. It also replaces the old tests with new ones.

- [ ] **Step 2: Update domain/mod.rs exports**

In `src/domain/mod.rs`, change:
```rust
#[allow(unused_imports)]
pub use event::{Event, YakEvent};
```
To:
```rust
#[allow(unused_imports)]
pub use event::YakEvent;
```

- [ ] **Step 3: Run `cargo check` and fix all compile errors**

Run: `cargo check 2>&1`

The compiler will list every file that needs updating. For each:

**Pattern for creation sites** (domain/yak.rs, domain/yak_map.rs,
application/prune_yaks.rs):
```rust
// Before:
YakEvent::Added { name: "foo".to_string() }
// After:
YakEvent::Added(AddedEvent { name: "foo".to_string() })

// Before:
YakEvent::StateUpdated { name: n.clone(), state: s.clone() }
// After:
YakEvent::StateUpdated(StateUpdatedEvent { name: n.clone(), state: s.clone() })
```

Each file will need an import at the top:
```rust
use crate::domain::events::*;
// or specific imports like:
use crate::domain::{AddedEvent, StateUpdatedEvent};
```

**Pattern for match sites** (storage/directory.rs, storage/memory.rs,
event_store/memory.rs, log/git_log.rs):
```rust
// Before:
YakEvent::Added { name } => { ... }
// After:
YakEvent::Added(AddedEvent { name }) => { ... }

// Before:
YakEvent::ContextUpdated { name, content } => { ... }
// After:
YakEvent::ContextUpdated(ContextUpdatedEvent { name, content }) => { ... }
```

**Pattern for filter expressions** (event_store/memory.rs):

Replace the match-based filter with the new `yak_name()` helper:
```rust
// Before:
.filter(|e| match e {
    YakEvent::Added { name: n } => n == name,
    YakEvent::Removed { name: n } => n == name,
    // ... 6 arms
})
// After:
.filter(|e| e.yak_name() == name)
```

**For test files in all the above**: Apply the same
creation/match patterns.

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass (same count, just updated syntax)

- [ ] **Step 5: Run lint**

Run: `cargo clippy -- -D warnings && cargo fmt`

- [ ] **Step 6: Commit**

```bash
git mit me && git add src/
git commit -m "Refactor YakEvent to wrap event structs"
```

---

## Chunk 2: GitEventStore + Contract Tests

### File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/adapters/event_store/contract_tests.rs` | Create | Macro-based contract tests |
| `src/adapters/event_store/git.rs` | Create | GitEventStore adapter |
| `src/adapters/event_store/mod.rs` | Modify | Register new modules |

---

### Task 4: Contract test macro for EventStore

**Files:**
- Create: `src/adapters/event_store/contract_tests.rs`
- Modify: `src/adapters/event_store/mod.rs`

- [ ] **Step 1: Create contract test macro**

Create `src/adapters/event_store/contract_tests.rs`:

```rust
/// Contract tests that must pass for all EventStore implementations.
/// Use the event_store_tests! macro to run against any implementation.
///
/// Note: The macro accepts an expression that returns `(impl EventStore, _guard)`.
/// The `_guard` keeps any resources (like TempDir) alive for the test duration.
/// For implementations that don't need a guard, pass `()`.
///
/// Content fields (ContextUpdatedEvent.content, FieldUpdatedEvent.content) may
/// be empty when read back from implementations that store content in trees
/// rather than commit messages. Tests only check event count, not content equality.
macro_rules! event_store_tests {
    ($create_store:expr) => {
        use crate::domain::{
            AddedEvent, ContextUpdatedEvent, FieldUpdatedEvent,
            MovedEvent, RemovedEvent, StateUpdatedEvent, YakEvent,
        };
        use crate::ports::EventStore;

        #[test]
        fn appends_and_retrieves_single_event() {
            let (mut store, _guard) = $create_store;
            let event = YakEvent::Added(AddedEvent {
                name: "foo".to_string(),
            });
            store.append(&event).unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 1);
        }

        #[test]
        fn appends_multiple_events() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "foo".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "bar".to_string(),
                }))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 2);
        }

        #[test]
        fn returns_events_in_chronological_order() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "first".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "second".to_string(),
                }))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all[0].yak_name(), "first");
            assert_eq!(all[1].yak_name(), "second");
        }

        #[test]
        fn filters_events_by_yak_name() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "foo".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "bar".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                    name: "foo".to_string(),
                    state: "wip".to_string(),
                }))
                .unwrap();

            let foo_events = store.get_events("foo").unwrap();
            assert_eq!(foo_events.len(), 2);

            let bar_events = store.get_events("bar").unwrap();
            assert_eq!(bar_events.len(), 1);

            let baz_events = store.get_events("baz").unwrap();
            assert_eq!(baz_events.len(), 0);
        }

        #[test]
        fn returns_empty_when_no_events() {
            let (store, _guard) = $create_store;
            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 0);
        }

        #[test]
        fn roundtrips_all_event_types() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "test".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                    name: "test".to_string(),
                    state: "wip".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Moved(MovedEvent {
                    old_name: "test".to_string(),
                    new_name: "test2".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::ContextUpdated(ContextUpdatedEvent {
                    name: "test2".to_string(),
                    content: "some context".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::FieldUpdated(FieldUpdatedEvent {
                    name: "test2".to_string(),
                    field_name: "notes".to_string(),
                    content: "stuff".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Removed(RemovedEvent {
                    name: "test2".to_string(),
                }))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 6);
        }
    };
}

pub(crate) use event_store_tests;
```

- [ ] **Step 2: Wire contract tests for InMemoryEventStore**

In `src/adapters/event_store/mod.rs`, change to:
```rust
pub mod memory;
pub use memory::InMemoryEventStore;

#[cfg(test)]
mod contract_tests;

#[cfg(test)]
mod in_memory_contract {
    use super::contract_tests::event_store_tests;
    event_store_tests!((super::InMemoryEventStore::new(), ()));
}
```

- [ ] **Step 3: Run contract tests**

Run: `cargo test in_memory_contract`
Expected: All 6 contract tests pass

- [ ] **Step 4: Commit**

```bash
git mit me && git add src/adapters/event_store/contract_tests.rs \
  src/adapters/event_store/mod.rs
git commit -m "Add contract tests for EventStore"
```

---

### Task 5: GitEventStore - basic structure + append

**Files:**
- Create: `src/adapters/event_store/git.rs`
- Modify: `src/adapters/event_store/mod.rs`

This is the core implementation. GitEventStore uses git2 to build
trees purely in `.git/objects/` without touching the filesystem.

Reference: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects

Key git2 APIs:
- `repo.blob(content)` - create blob from bytes
- `repo.treebuilder(base_tree)` - create tree builder
- `treebuilder.insert(name, oid, filemode)` - add entry to tree
- `treebuilder.write()` - write tree to object database
- `repo.commit(ref, sig, sig, msg, tree, parents)` - create commit

- [ ] **Step 1: Create GitEventStore skeleton with test**

Create `src/adapters/event_store/git.rs`:

```rust
use anyhow::{Context, Result};
use git2::Repository;
use std::path::{Path, PathBuf};

use crate::domain::YakEvent;
use crate::ports::EventStore;

pub struct GitEventStore {
    repo: Repository,
}

impl GitEventStore {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo = Repository::open(repo_path)
            .with_context(|| format!("Failed to open git repo at {}", repo_path.display()))?;
        Ok(Self { repo })
    }

    /// For tests: create from an already-opened Repository
    #[cfg(test)]
    pub fn from_repo(repo: Repository) -> Self {
        Self { repo }
    }
}

impl EventStore for GitEventStore {
    fn append(&mut self, _event: &YakEvent) -> Result<()> {
        todo!()
    }

    fn get_events(&self, _name: &str) -> Result<Vec<YakEvent>> {
        todo!()
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AddedEvent;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitEventStore) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        // Configure git user for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        let store = GitEventStore::from_repo(repo);
        (tmp, store)
    }

    #[test]
    fn append_creates_commit_on_refs_notes_yaks() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
            }))
            .unwrap();

        // Verify ref exists
        let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = store.repo.find_commit(oid).unwrap();
        assert_eq!(commit.message().unwrap(), "Added: \"test\"");
    }

    #[test]
    fn append_builds_tree_with_yak_directory() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
            }))
            .unwrap();

        let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = store.repo.find_commit(oid).unwrap();
        let tree = commit.tree().unwrap();

        // Verify test/ directory exists in tree
        let entry = tree.get_name("test").unwrap();
        let subtree = entry.to_object(&store.repo).unwrap();
        let subtree = subtree.as_tree().unwrap();

        // Verify state file
        let state_entry = subtree.get_name("state").unwrap();
        let state_blob = state_entry.to_object(&store.repo).unwrap();
        let state_content = std::str::from_utf8(
            state_blob.as_blob().unwrap().content(),
        )
        .unwrap();
        assert_eq!(state_content, "todo");
    }
}
```

- [ ] **Step 2: Run test to verify it fails (RED)**

Run: `cargo test event_store::git::tests`
Expected: FAIL with "not yet implemented"

- [ ] **Step 3: Implement tree building helpers and append**

Add these methods to `GitEventStore`:

```rust
impl GitEventStore {
    // ... existing new/from_repo ...

    /// Get the latest commit on refs/notes/yaks, if any
    fn get_latest_commit(&self) -> Result<Option<git2::Commit>> {
        match self.repo.refname_to_id("refs/notes/yaks") {
            Ok(oid) => Ok(Some(self.repo.find_commit(oid)?)),
            Err(_) => Ok(None),
        }
    }

    /// Get the current tree from refs/notes/yaks, if any
    fn get_current_tree(&self) -> Result<Option<git2::Tree>> {
        match self.get_latest_commit()? {
            Some(commit) => Ok(Some(commit.tree()?)),
            None => Ok(None),
        }
    }

    /// Create a tree for a single yak with initial files
    fn create_yak_tree(&self, state: &str, context: &str) -> Result<git2::Oid> {
        let mut builder = self.repo.treebuilder(None)?;

        let state_blob = self.repo.blob(state.as_bytes())?;
        builder.insert("state", state_blob, 0o100644)?;

        let context_blob = self.repo.blob(context.as_bytes())?;
        builder.insert("context.md", context_blob, 0o100644)?;

        Ok(builder.write()?)
    }

    /// Get a yak's subtree from the root tree
    fn get_yak_subtree(
        &self,
        root: Option<&git2::Tree>,
        yak_name: &str,
    ) -> Result<Option<git2::Tree<'_>>> {
        let Some(root) = root else {
            return Ok(None);
        };

        let parts: Vec<&str> = yak_name.split('/').collect();
        let mut current_tree = root.clone();

        for part in &parts {
            match current_tree.get_name(part) {
                Some(entry) => {
                    let obj = entry.to_object(&self.repo)?;
                    current_tree = obj.into_tree().map_err(|_| {
                        anyhow::anyhow!("Expected tree entry for '{}'", part)
                    })?;
                }
                None => return Ok(None),
            }
        }

        // Re-fetch to get owned Tree with correct lifetime
        let oid = current_tree.id();
        Ok(Some(self.repo.find_tree(oid)?))
    }

    /// Update a file in a yak's subtree, returning new root tree OID
    fn update_yak_file(
        &self,
        current_tree: Option<&git2::Tree>,
        yak_name: &str,
        file_name: &str,
        content: &str,
    ) -> Result<git2::Oid> {
        let blob_oid = self.repo.blob(content.as_bytes())?;

        // Build the yak's subtree
        let yak_subtree = self.get_yak_subtree(current_tree, yak_name)?;
        let mut yak_builder = self.repo.treebuilder(yak_subtree.as_ref())?;
        yak_builder.insert(file_name, blob_oid, 0o100644)?;
        let yak_tree_oid = yak_builder.write()?;

        // Rebuild root tree with updated yak subtree
        self.set_yak_in_root(current_tree, yak_name, Some(yak_tree_oid))
    }

    /// Set (or remove) a yak subtree in the root tree, handling
    /// hierarchical names by rebuilding intermediate trees.
    fn set_yak_in_root(
        &self,
        root: Option<&git2::Tree>,
        yak_name: &str,
        subtree_oid: Option<git2::Oid>,
    ) -> Result<git2::Oid> {
        let parts: Vec<&str> = yak_name.split('/').collect();

        if parts.len() == 1 {
            // Simple case: direct child of root
            let mut builder = self.repo.treebuilder(root)?;
            match subtree_oid {
                Some(oid) => {
                    builder.insert(parts[0], oid, 0o040000)?;
                }
                None => {
                    let _ = builder.remove(parts[0]);
                }
            }
            return Ok(builder.write()?);
        }

        // Hierarchical case: need to rebuild intermediate trees
        let intermediate_name = parts[0];
        let rest = parts[1..].join("/");

        let intermediate_tree = root
            .and_then(|r| r.get_name(intermediate_name))
            .map(|entry| self.repo.find_tree(entry.id()))
            .transpose()?;

        let new_intermediate = self.set_yak_in_root(
            intermediate_tree.as_ref(),
            &rest,
            subtree_oid,
        )?;

        let mut root_builder = self.repo.treebuilder(root)?;
        root_builder.insert(intermediate_name, new_intermediate, 0o040000)?;
        Ok(root_builder.write()?)
    }

    /// Build an updated tree by applying an event to the current tree.
    /// All operations happen in git's object database - no filesystem IO.
    fn build_tree_from_event(
        &self,
        event: &YakEvent,
        current_tree: Option<&git2::Tree>,
    ) -> Result<git2::Oid> {
        match event {
            YakEvent::Added(e) => {
                let yak_tree_oid = self.create_yak_tree("todo", "")?;
                self.set_yak_in_root(current_tree, &e.name, Some(yak_tree_oid))
            }

            YakEvent::Removed(e) => {
                self.set_yak_in_root(current_tree, &e.name, None)
            }

            YakEvent::Moved(e) => {
                // Get old subtree
                let old_subtree_oid = self
                    .get_yak_subtree(current_tree, &e.old_name)?
                    .map(|t| t.id());

                // Remove old, add new
                let intermediate = self.set_yak_in_root(
                    current_tree, &e.old_name, None,
                )?;
                let intermediate_tree = self.repo.find_tree(intermediate)?;
                self.set_yak_in_root(
                    Some(&intermediate_tree),
                    &e.new_name,
                    old_subtree_oid,
                )
            }

            YakEvent::ContextUpdated(e) => {
                self.update_yak_file(
                    current_tree, &e.name, "context.md", &e.content,
                )
            }

            YakEvent::StateUpdated(e) => {
                self.update_yak_file(
                    current_tree, &e.name, "state", &e.state,
                )
            }

            YakEvent::FieldUpdated(e) => {
                self.update_yak_file(
                    current_tree, &e.name, &e.field_name, &e.content,
                )
            }
        }
    }
}

impl EventStore for GitEventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()> {
        let current_tree = self.get_current_tree()?;

        let tree_oid = self.build_tree_from_event(
            event,
            current_tree.as_ref(),
        )?;
        let tree = self.repo.find_tree(tree_oid)?;

        let message = event.format_message();

        let parent = self.get_latest_commit()?;
        let parents: Vec<&git2::Commit> =
            parent.iter().collect();

        let sig = self.repo.signature()
            .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;

        self.repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            &message,
            &tree,
            &parents,
        )?;

        Ok(())
    }

    fn get_events(&self, _name: &str) -> Result<Vec<YakEvent>> {
        todo!("Implemented in Task 6")
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        todo!("Implemented in Task 6")
    }
}
```

- [ ] **Step 4: Run tests (GREEN)**

Run: `cargo test event_store::git::tests`
Expected: Both tests pass

- [ ] **Step 5: Register module**

In `src/adapters/event_store/mod.rs`, add:
```rust
pub mod git;
pub use git::GitEventStore;
```

And in `src/adapters/mod.rs`, add to re-exports:
```rust
#[allow(unused_imports)]
pub use event_store::GitEventStore;
```

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: All pass

- [ ] **Step 7: Commit**

```bash
git mit me && git add src/adapters/event_store/git.rs \
  src/adapters/event_store/mod.rs src/adapters/mod.rs
git commit -m "Add GitEventStore with append and tree building"
```

---

### Task 6: GitEventStore - reading events + contract tests

**Files:**
- Modify: `src/adapters/event_store/git.rs`
- Modify: `src/adapters/event_store/mod.rs`

- [ ] **Step 1: Write failing test for get_all_events**

The contract tests for GitEventStore will serve as the failing tests.
Add to `src/adapters/event_store/mod.rs`:

```rust
#[cfg(test)]
mod git_contract {
    use super::contract_tests::event_store_tests;
    use tempfile::TempDir;
    use git2::Repository;

    fn create_git_store() -> (super::GitEventStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (super::GitEventStore::from_repo(repo), tmp)
    }

    event_store_tests!(create_git_store());
}
```

- [ ] **Step 2: Run contract tests to verify RED**

Run: `cargo test git_contract`
Expected: FAIL with "not yet implemented"

- [ ] **Step 3: Implement get_all_events and get_events**

In `src/adapters/event_store/git.rs`, replace the `todo!()` methods:

```rust
    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        let Some(latest) = self.get_latest_commit()? else {
            return Ok(Vec::new());
        };

        let mut events = Vec::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        revwalk.push(latest.id())?;

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let message = commit.message().unwrap_or("").trim();

            if message.is_empty() {
                continue;
            }

            match YakEvent::parse(message) {
                Ok(event) => events.push(event),
                Err(_) => continue, // Skip unparseable commits
            }
        }

        // Reverse: revwalk gives newest-first, we want chronological
        events.reverse();
        Ok(events)
    }

    fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
        // Walk all events and filter by yak name.
        // Could optimize with git log message grep later.
        Ok(self
            .get_all_events()?
            .into_iter()
            .filter(|e| e.yak_name() == name)
            .collect())
    }
```

- [ ] **Step 4: Run all contract tests (GREEN)**

Run: `cargo test contract`
Expected: 12 tests pass (6 for InMemory, 6 for Git)

- [ ] **Step 5: Commit**

```bash
git mit me && git add src/adapters/event_store/git.rs \
  src/adapters/event_store/mod.rs
git commit -m "Add event reading to GitEventStore"
```

---

## Chunk 3: Wiring + Cleanup

### Task 7: Update ShellSpec tests for new format

**Important:** Update tests BEFORE changing production code (TDD).
The tests will fail against the old implementation -- that's expected.
They define the new behavior we want.

**Files:**
- Modify: `spec/features/log.sh`
- Modify: `spec/unit/log_command.sh`

- [ ] **Step 1: Update spec/features/log.sh**

Replace the file with:

```bash
# shellcheck shell=bash
Describe 'yx log'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'shows empty log when no events exist'
    When run yx log
    The output should equal ""
    The status should be success
  End

  It 'displays add events'
    When run sh -c "
      yx add 'test yak'
      yx log
    "
    The output should include 'Added: "test yak"'
    The status should be success
  End

  It 'displays events in chronological order'
    When run sh -c "
      yx add 'first yak'
      yx add 'second yak'
      yx log
    "
    The line 1 of output should include 'Added: "first yak"'
    The line 2 of output should include 'Added: "second yak"'
    The status should be success
  End

  It 'displays done events'
    When run sh -c "
      yx add 'test yak'
      yx done 'test yak'
      yx log
    "
    The line 1 of output should include 'Added: "test yak"'
    The line 2 of output should include 'StateUpdated: "test yak" "done"'
    The status should be success
  End

  It 'displays done --undo events'
    When run sh -c "
      yx add 'test yak'
      yx done 'test yak'
      yx done --undo 'test yak'
      yx log
    "
    The line 1 of output should include 'Added: "test yak"'
    The line 2 of output should include 'StateUpdated: "test yak" "done"'
    The line 3 of output should include 'StateUpdated: "test yak" "todo"'
    The status should be success
  End

  It 'displays remove events'
    When run sh -c "
      yx add 'test yak'
      yx rm 'test yak'
      yx log
    "
    The line 1 of output should include 'Added: "test yak"'
    The line 2 of output should include 'Removed: "test yak"'
    The status should be success
  End

  It 'displays context events'
    When run sh -c "
      unset YX_IGNORE_STDIN
      yx add 'test yak'
      echo 'Some context' | yx context 'test yak'
      yx log
    "
    The line 1 of output should include 'Added: "test yak"'
    The line 2 of output should include 'ContextUpdated: "test yak"'
    The status should be success
  End
End
```

Note: The timestamp and author tests are removed because the new
format no longer includes them in `yx log` output.

- [ ] **Step 2: Update spec/unit/log_command.sh**

Replace the file with:

```bash
# shellcheck shell=bash
Describe 'log_command'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'commits yak changes to refs/notes/yaks'
    When run in_test_repo "
      yx add 'test yak'
      # Check that refs/notes/yaks exists and has a commit
      git rev-parse refs/notes/yaks >/dev/null 2>&1
    "
    The status should be success
  End

  It 'uses tagged commit message for add command'
    When run in_test_repo "
      yx add 'test yak'
      git log refs/notes/yaks -1 --format=%s
    "
    The output should equal 'Added: "test yak"'
  End

  It 'includes git author in commits'
    When run in_test_repo "
      yx add 'test yak'
      git log refs/notes/yaks -1 --format='%an <%ae>'
    "
    The output should equal "Test User <test@example.com>"
  End

  It 'creates sequential commits on multiple operations'
    When run in_test_repo "
      yx add 'yak one'
      yx add 'yak two'
      git log refs/notes/yaks --oneline | wc -l
    "
    The output should equal "2"
  End

  It 'done command creates commit with tagged message'
    When run in_test_repo "
      yx add 'test yak'
      yx done 'test yak'
      git log refs/notes/yaks -1 --format=%s
    "
    The output should equal 'StateUpdated: "test yak" "done"'
  End

  It 'done --undo command creates commit with tagged message'
    When run in_test_repo "
      yx add 'test yak'
      yx done 'test yak'
      yx done --undo 'test yak'
      git log refs/notes/yaks -1 --format=%s
    "
    The output should equal 'StateUpdated: "test yak" "todo"'
  End

  It 'logs removal even when yaks path becomes empty'
    When run in_test_repo "
      yx add 'only yak'
      yx rm 'only yak'
      git log refs/notes/yaks -1 --format=%s
    "
    The output should equal 'Removed: "only yak"'
  End
End
```

- [ ] **Step 3: Commit**

```bash
git mit me && git add spec/features/log.sh spec/unit/log_command.sh
git commit -m "Update log specs for new tagged event format"
```

---

### Task 8: Wire GitEventStore into main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Replace InMemoryEventStore with GitEventStore**

In `src/main.rs`, change the event infrastructure setup.

Replace:
```rust
use adapters::InMemoryEventStore;
```
With:
```rust
use adapters::event_store::GitEventStore;
use std::path::PathBuf;
```

Replace:
```rust
let event_store = InMemoryEventStore::new();
let mut event_bus = EventBus::new(Box::new(event_store));
```
With:
```rust
// Determine repo path: GIT_WORK_TREE env var, then current dir
let repo_path = std::env::var("GIT_WORK_TREE")
    .map(PathBuf::from)
    .unwrap_or(std::env::current_dir()?);
let event_store = GitEventStore::new(&repo_path)?;
let mut event_bus = EventBus::new(Box::new(event_store));
```

- [ ] **Step 2: Remove GitLog registration as EventListener**

Remove these lines from `main.rs`:
```rust
let log = GitLog::new()?;
event_bus.register(Box::new(log.clone()));
```

And remove:
```rust
use adapters::log::GitLog;
```

- [ ] **Step 3: Update yx log command**

Replace the `Commands::Log` handler with:
```rust
Commands::Log => {
    let repo_path = std::env::var("GIT_WORK_TREE")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let reader = GitEventStore::new(&repo_path)?;
    let events = reader.get_all_events()?;
    for event in events {
        println!("{}", event.format_message());
    }
    Ok(())
}
```

- [ ] **Step 4: Build release binary and run ShellSpec**

Run:
```bash
cargo build --release
cd /Users/mattwynne/git/mattwynne/yaks && shellspec spec/features/log.sh spec/unit/log_command.sh
```
Expected: All tests pass

- [ ] **Step 5: Run all checks**

Run: `cd /Users/mattwynne/git/mattwynne/yaks && dev check`
Expected: All checks pass

- [ ] **Step 6: Commit**

```bash
git mit me && git add src/main.rs
git commit -m "Wire GitEventStore into main, remove GitLog"
```

---

### Task 9: Remove LogPort, GitLog, legacy Event

**Files:**
- Delete: `src/ports/log.rs`
- Delete: `src/adapters/log/git_log.rs`
- Delete: `src/adapters/log/memory.rs`
- Delete: `src/adapters/log/mod.rs`
- Modify: `src/ports/mod.rs` - remove log module
- Modify: `src/adapters/mod.rs` - remove log module + InMemoryLog export
- Modify: `tests/features/in_process_world.rs` - remove InMemoryLog
- Modify: `Cargo.toml` - remove `chrono` if unused

- [ ] **Step 1: Remove log port and adapters**

Delete these files:
- `src/ports/log.rs`
- `src/adapters/log/git_log.rs`
- `src/adapters/log/memory.rs`
- `src/adapters/log/mod.rs`

Update `src/ports/mod.rs`: remove `pub mod log;` and
`pub use log::LogPort;`

Update `src/adapters/mod.rs`: remove `pub mod log;` and
`pub use log::{GitLog, InMemoryLog};`

- [ ] **Step 2: Update in_process_world.rs**

In `tests/features/in_process_world.rs`:
- Remove `InMemoryLog` from imports (`use yx::adapters::{..., InMemoryLog, ...}`)
- Remove `log: InMemoryLog` field from struct
- Remove `log: InMemoryLog::new()` from constructor
- Remove `#[allow(dead_code)]` on the log field

- [ ] **Step 3: Check if chrono can be removed**

Run: `cargo check 2>&1`. If `chrono` is no longer used anywhere
(the `Event` struct and `GitLog` were the only users), remove
`chrono` from `[dependencies]` in `Cargo.toml`.

- [ ] **Step 4: Build and run all tests**

Run: `cargo test`
Then: `cargo build --release`
Then: `cd /Users/mattwynne/git/mattwynne/yaks && dev check`
Expected: All pass

- [ ] **Step 5: Commit**

```bash
git mit me && git add src/ports/mod.rs src/adapters/mod.rs \
  tests/features/in_process_world.rs Cargo.toml Cargo.lock
git rm src/ports/log.rs src/adapters/log/git_log.rs \
  src/adapters/log/memory.rs src/adapters/log/mod.rs
git commit -m "Remove LogPort, GitLog, and legacy Event struct"
```

---

### Task 10: Snapshot initialization for DirectoryStorage

**Files:**
- Modify: `src/adapters/storage/directory.rs`

This enables DirectoryStorage to initialize from a git tree snapshot
rather than starting empty.

- [ ] **Step 1: Write test for snapshot initialization**

Add to `src/adapters/storage/directory.rs` tests:

```rust
#[test]
fn test_new_from_snapshot() {
    use crate::adapters::event_store::GitEventStore;
    use crate::domain::{AddedEvent, YakEvent};
    use crate::ports::Store;
    use git2::Repository;

    let tmp = TempDir::new().unwrap();
    let repo = Repository::init(tmp.path()).unwrap();
    // Configure git
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "test").unwrap();
    config.set_str("user.email", "test@test.com").unwrap();

    // Create a GitEventStore and add a yak
    let mut event_store = GitEventStore::from_repo(
        Repository::open(tmp.path()).unwrap()
    );
    event_store.append(&YakEvent::Added(AddedEvent {
        name: "test".to_string(),
    })).unwrap();

    // Create DirectoryStorage from snapshot
    let yak_path = tmp.path().join(".yaks");
    let storage = DirectoryStorage::new_from_snapshot(
        &yak_path,
        &Repository::open(tmp.path()).unwrap(),
    ).unwrap();

    // Verify yak exists in directory
    assert!(yak_path.join("test").join("state").exists());
    assert!(Store::yak_exists(&storage, "test"));
}
```

- [ ] **Step 2: Run test to verify it fails (RED)**

Run: `cargo test test_new_from_snapshot`
Expected: FAIL (method doesn't exist yet)

- [ ] **Step 3: Implement new_from_snapshot**

Add to `DirectoryStorage` impl block:

```rust
    /// Creates a DirectoryStorage initialized from the latest git tree
    /// on refs/notes/yaks. This materializes the tree into the filesystem
    /// so DirectoryStorage can serve reads immediately.
    pub fn new_from_snapshot(
        yak_path: &Path,
        repo: &git2::Repository,
    ) -> Result<Self> {
        // Create directory if needed
        std::fs::create_dir_all(yak_path)?;

        // Read latest tree from refs/notes/yaks
        if let Ok(oid) = repo.refname_to_id("refs/notes/yaks") {
            let commit = repo.find_commit(oid)?;
            let tree = commit.tree()?;
            Self::materialize_tree(yak_path, &tree, repo)?;
        }

        Ok(Self {
            base_path: yak_path.to_path_buf(),
        })
    }

    fn materialize_tree(
        base_path: &Path,
        tree: &git2::Tree,
        repo: &git2::Repository,
    ) -> Result<()> {
        for entry in tree.iter() {
            let name = entry.name()
                .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in tree entry"))?;
            let path = base_path.join(name);

            match entry.kind() {
                Some(git2::ObjectType::Tree) => {
                    std::fs::create_dir_all(&path)?;
                    let subtree = repo.find_tree(entry.id())?;
                    Self::materialize_tree(&path, &subtree, repo)?;
                }
                Some(git2::ObjectType::Blob) => {
                    let blob = repo.find_blob(entry.id())?;
                    std::fs::write(&path, blob.content())?;
                }
                _ => {}
            }
        }
        Ok(())
    }
```

Add `use std::path::Path;` to the imports at the top of the file if
not already present.

- [ ] **Step 4: Run test (GREEN)**

Run: `cargo test test_new_from_snapshot`
Expected: Test passes

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

- [ ] **Step 6: Run dev check**

Run: `cd /Users/mattwynne/git/mattwynne/yaks && dev check`
Expected: All checks pass

- [ ] **Step 7: Commit**

```bash
git mit me && git add src/adapters/storage/directory.rs
git commit -m "Add snapshot initialization for DirectoryStorage"
```

---

## Summary

| Chunk | Tasks | What it delivers |
|-------|-------|-----------------|
| 1 | Tasks 1-3 | EventFormat trait + YakEvent refactoring |
| 2 | Tasks 4-6 | GitEventStore with contract tests |
| 3 | Tasks 7-10 | Production wiring + cleanup + snapshot |

Total: 10 tasks, ~40 steps
