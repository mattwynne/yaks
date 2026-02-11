# Event-Sourced Architecture Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor yak to use event sourcing with CQRS and DDD patterns, moving business logic into rich domain aggregates.

**Architecture:** Events are the source of truth stored in EventStore. Storage becomes an event-driven projection implementing both EventListener (write) and Store (read). Application provides closure-based API (with_yak) that encapsulates load→mutate→save pattern.

**Tech Stack:** Rust, existing hexagonal architecture, TDD with unit tests + Cucumber

---

## Phase 1: Event Infrastructure

### Task 1: Create YakEvent Enum

**Files:**
- Modify: `src/domain/event.rs`
- Test: Unit tests in same file

**Step 1: Write failing test for YakEvent enum**

Add to `src/domain/event.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_added_event() {
        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        match event {
            YakEvent::Added { name } => assert_eq!(name, "test"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_context_updated_event() {
        let event = YakEvent::ContextUpdated {
            name: "test".to_string(),
            content: "context".to_string(),
        };

        match event {
            YakEvent::ContextUpdated { name, content } => {
                assert_eq!(name, "test");
                assert_eq!(content, "context");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_state_updated_event() {
        let event = YakEvent::StateUpdated {
            name: "test".to_string(),
            state: "wip".to_string(),
        };

        match event {
            YakEvent::StateUpdated { name, state } => {
                assert_eq!(name, "test");
                assert_eq!(state, "wip");
            }
            _ => panic!("Wrong event type"),
        }
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test '*' event
```

Expected: Compilation error - "YakEvent::Added not found"

**Step 3: Implement YakEvent enum**

Replace the existing Event struct in `src/domain/event.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum YakEvent {
    Added {
        name: String,
    },

    Removed {
        name: String,
    },

    Moved {
        old_name: String,
        new_name: String,
    },

    ContextUpdated {
        name: String,
        content: String,
    },

    StateUpdated {
        name: String,
        state: String,
    },

    FieldUpdated {
        name: String,
        field_name: String,
        content: String,
    },
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test --test '*' event
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/domain/event.rs
git commit -m "Define YakEvent enum with domain events

Replace Event struct with YakEvent enum containing:
- Added, Removed, Moved (lifecycle)
- ContextUpdated, StateUpdated, FieldUpdated (attributes)

Events contain only domain data, no timestamps."
```

---

### Task 2: Create EventListener Trait

**Files:**
- Create: `src/ports/event_listener.rs`
- Modify: `src/ports/mod.rs`

**Step 1: Write failing test for EventListener trait**

Create `src/ports/event_listener.rs`:

```rust
use anyhow::Result;
use crate::domain::YakEvent;

pub trait EventListener {
    fn on_event(&mut self, event: &YakEvent) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestListener {
        events: Vec<YakEvent>,
    }

    impl EventListener for TestListener {
        fn on_event(&mut self, event: &YakEvent) -> Result<()> {
            self.events.push(event.clone());
            Ok(())
        }
    }

    #[test]
    fn test_event_listener_receives_events() {
        let mut listener = TestListener { events: vec![] };

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        listener.on_event(&event).unwrap();

        assert_eq!(listener.events.len(), 1);
        assert_eq!(listener.events[0], event);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib event_listener
```

Expected: Compilation error - module not found

**Step 3: Add module to ports**

Add to `src/ports/mod.rs`:

```rust
pub mod event_listener;
pub use event_listener::EventListener;
```

**Step 4: Run test to verify it passes**

```bash
cargo test --lib event_listener
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/ports/event_listener.rs src/ports/mod.rs
git commit -m "Add EventListener trait for event projections

EventListener trait allows infrastructure to react to
domain events. Projections implement this to update
their state."
```

---

### Task 3: Create Store Trait (Read Model)

**Files:**
- Create: `src/ports/store.rs`
- Modify: `src/ports/mod.rs`

**Step 1: Write failing test for Store trait**

Create `src/ports/store.rs`:

```rust
use anyhow::Result;
use crate::domain::Yak;

pub trait Store {
    fn get_yak(&self, name: &str) -> Result<Yak>;
    fn list_yaks(&self) -> Result<Vec<Yak>>;
    fn yak_exists(&self, name: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct InMemoryStore {
        yaks: HashMap<String, Yak>,
    }

    impl Store for InMemoryStore {
        fn get_yak(&self, name: &str) -> Result<Yak> {
            self.yaks
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Yak not found"))
        }

        fn list_yaks(&self) -> Result<Vec<Yak>> {
            Ok(self.yaks.values().cloned().collect())
        }

        fn yak_exists(&self, name: &str) -> bool {
            self.yaks.contains_key(name)
        }
    }

    #[test]
    fn test_store_get_yak() {
        let mut yaks = HashMap::new();
        yaks.insert(
            "test".to_string(),
            Yak {
                name: "test".to_string(),
                state: "todo".to_string(),
                context: None,
                pending_events: vec![],
            },
        );

        let store = InMemoryStore { yaks };
        let yak = store.get_yak("test").unwrap();

        assert_eq!(yak.name, "test");
    }

    #[test]
    fn test_store_yak_exists() {
        let mut yaks = HashMap::new();
        yaks.insert(
            "test".to_string(),
            Yak {
                name: "test".to_string(),
                state: "todo".to_string(),
                context: None,
                pending_events: vec![],
            },
        );

        let store = InMemoryStore { yaks };

        assert!(store.yak_exists("test"));
        assert!(!store.yak_exists("missing"));
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib store
```

Expected: Compilation error - "pending_events field not found on Yak"

**Step 3: Add module to ports**

Add to `src/ports/mod.rs`:

```rust
pub mod store;
pub use store::Store;
```

**Step 4: Run test (will still fail until we add pending_events to Yak)**

```bash
cargo test --lib store
```

Expected: Compilation error - "pending_events field not found"

**Step 5: Temporarily skip this commit - we need to update Yak first**

We'll commit this after updating Yak in Phase 2.

---

## Phase 2: Domain Model Updates

### Task 4: Update Yak Aggregate

**Files:**
- Modify: `src/domain/yak.rs`
- Update tests in same file

**Step 1: Write tests for updated Yak**

Add to `src/domain/yak.rs` tests:

```rust
#[test]
fn test_yak_emits_added_event() {
    let mut yak = Yak::new("test".to_string());
    let events = yak.take_events();

    assert_eq!(events.len(), 1);
    match &events[0] {
        YakEvent::Added { name } => assert_eq!(name, "test"),
        _ => panic!("Expected Added event"),
    }
}

#[test]
fn test_yak_is_done_derived_from_state() {
    let mut yak = Yak::new("test".to_string());
    assert!(!yak.is_done());

    yak.state = "done".to_string();
    assert!(yak.is_done());
}

#[test]
fn test_yak_update_context_emits_event() {
    let mut yak = Yak::new("test".to_string());
    yak.take_events(); // clear creation event

    yak.update_context("new context".to_string()).unwrap();
    let events = yak.take_events();

    assert_eq!(events.len(), 1);
    match &events[0] {
        YakEvent::ContextUpdated { name, content } => {
            assert_eq!(name, "test");
            assert_eq!(content, "new context");
        }
        _ => panic!("Expected ContextUpdated event"),
    }
}

#[test]
fn test_yak_update_state_emits_event() {
    let mut yak = Yak::new("test".to_string());
    yak.take_events(); // clear creation event

    yak.update_state("wip".to_string()).unwrap();
    let events = yak.take_events();

    assert_eq!(events.len(), 1);
    match &events[0] {
        YakEvent::StateUpdated { name, state } => {
            assert_eq!(name, "test");
            assert_eq!(state, "wip");
        }
        _ => panic!("Expected StateUpdated event"),
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test --lib yak
```

Expected: Compilation errors - methods not found

**Step 3: Update Yak struct**

Replace Yak struct in `src/domain/yak.rs`:

```rust
use crate::domain::YakEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yak {
    pub name: String,
    pub state: String,
    pub context: Option<String>,
    pub pending_events: Vec<YakEvent>,
}

impl Yak {
    pub fn new(name: String) -> Self {
        let mut yak = Self {
            name: name.clone(),
            state: "todo".to_string(),
            context: None,
            pending_events: vec![],
        };

        yak.pending_events.push(YakEvent::Added { name });
        yak
    }

    pub fn is_done(&self) -> bool {
        self.state == "done"
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_state(mut self, state: String) -> Self {
        self.state = state;
        self
    }

    pub fn update_context(&mut self, content: String) -> anyhow::Result<()> {
        self.context = Some(content.clone());
        self.pending_events.push(YakEvent::ContextUpdated {
            name: self.name.clone(),
            content,
        });
        Ok(())
    }

    pub fn update_state(&mut self, state: String) -> anyhow::Result<()> {
        self.state = state.clone();
        self.pending_events.push(YakEvent::StateUpdated {
            name: self.name.clone(),
            state,
        });
        Ok(())
    }

    pub fn take_events(&mut self) -> Vec<YakEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
```

**Step 4: Remove old methods that referenced `done` field**

Remove these methods from Yak impl:
- `mark_done()`
- `mark_undone()`

Update tests that used these methods to use `update_state()` instead.

**Step 5: Run tests to verify they pass**

```bash
cargo test --lib yak
```

Expected: All tests pass

**Step 6: Commit Yak changes**

```bash
git add src/domain/yak.rs
git commit -m "Refactor Yak to event-sourced aggregate

Changes:
- Remove done field (derive from state == \"done\")
- Add pending_events collection
- Add update_context(), update_state() that emit events
- Add take_events() to collect pending events
- new() emits Added event"
```

**Step 7: Now commit Store trait**

```bash
git add src/ports/store.rs src/ports/mod.rs
git commit -m "Add Store trait for read model queries

Store provides read interface to current yak state.
Projections implement this to answer queries."
```

**Step 8: Run all tests**

```bash
cargo test --lib
```

Expected: Some tests may fail due to Yak structure changes. We'll fix them incrementally.

---

### Task 5: Create EventStore Trait

**Files:**
- Create: `src/ports/event_store.rs`
- Modify: `src/ports/mod.rs`

**Step 1: Write EventStore trait with test**

Create `src/ports/event_store.rs`:

```rust
use anyhow::Result;
use crate::domain::YakEvent;

pub trait EventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()>;
    fn get_events(&self, name: &str) -> Result<Vec<YakEvent>>;
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InMemoryEventStore {
        events: Vec<YakEvent>,
    }

    impl EventStore for InMemoryEventStore {
        fn append(&mut self, event: &YakEvent) -> Result<()> {
            self.events.push(event.clone());
            Ok(())
        }

        fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
            Ok(self
                .events
                .iter()
                .filter(|e| match e {
                    YakEvent::Added { name: n } => n == name,
                    YakEvent::Removed { name: n } => n == name,
                    YakEvent::ContextUpdated { name: n, .. } => n == name,
                    YakEvent::StateUpdated { name: n, .. } => n == name,
                    YakEvent::Moved { old_name, .. } => old_name == name,
                    YakEvent::FieldUpdated { name: n, .. } => n == name,
                })
                .cloned()
                .collect())
        }

        fn get_all_events(&self) -> Result<Vec<YakEvent>> {
            Ok(self.events.clone())
        }
    }

    #[test]
    fn test_event_store_append() {
        let mut store = InMemoryEventStore { events: vec![] };

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        store.append(&event).unwrap();
        let events = store.get_all_events().unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn test_event_store_get_events_by_name() {
        let mut store = InMemoryEventStore { events: vec![] };

        store
            .append(&YakEvent::Added {
                name: "test1".to_string(),
            })
            .unwrap();
        store
            .append(&YakEvent::Added {
                name: "test2".to_string(),
            })
            .unwrap();
        store
            .append(&YakEvent::ContextUpdated {
                name: "test1".to_string(),
                content: "content".to_string(),
            })
            .unwrap();

        let events = store.get_events("test1").unwrap();

        assert_eq!(events.len(), 2);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib event_store
```

Expected: Module not found

**Step 3: Add module to ports**

Add to `src/ports/mod.rs`:

```rust
pub mod event_store;
pub use event_store::EventStore;
```

**Step 4: Run tests**

```bash
cargo test --lib event_store
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/ports/event_store.rs src/ports/mod.rs
git commit -m "Add EventStore trait for event persistence

EventStore is the source of truth for all domain events.
Provides append and query operations."
```

---

### Task 6: Create InMemoryEventStore Adapter

**Files:**
- Create: `src/adapters/event_store/memory.rs`
- Create: `src/adapters/event_store/mod.rs`
- Modify: `src/adapters/mod.rs`

**Step 1: Write InMemoryEventStore implementation**

Create `src/adapters/event_store/mod.rs`:

```rust
pub mod memory;
pub use memory::InMemoryEventStore;
```

Create `src/adapters/event_store/memory.rs`:

```rust
use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::domain::YakEvent;
use crate::ports::EventStore;

#[derive(Clone)]
pub struct InMemoryEventStore {
    events: Arc<Mutex<Vec<YakEvent>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl EventStore for InMemoryEventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }

    fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .iter()
            .filter(|e| match e {
                YakEvent::Added { name: n } => n == name,
                YakEvent::Removed { name: n } => n == name,
                YakEvent::ContextUpdated { name: n, .. } => n == name,
                YakEvent::StateUpdated { name: n, .. } => n == name,
                YakEvent::Moved { old_name, .. } => old_name == name,
                YakEvent::FieldUpdated { name: n, .. } => n == name,
            })
            .cloned()
            .collect())
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        Ok(self.events.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_event_store() {
        let mut store = InMemoryEventStore::new();

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        store.append(&event).unwrap();
        let events = store.get_all_events().unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib memory
```

Expected: Module not found

**Step 3: Add module to adapters**

Add to `src/adapters/mod.rs`:

```rust
pub mod event_store;
pub use event_store::InMemoryEventStore;
```

**Step 4: Run tests**

```bash
cargo test --lib memory
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/adapters/event_store/ src/adapters/mod.rs
git commit -m "Add InMemoryEventStore adapter

In-memory implementation of EventStore for testing
and initial development."
```

---

### Task 7: Create EventBus

**Files:**
- Create: `src/infrastructure/event_bus.rs`
- Create: `src/infrastructure/mod.rs`
- Modify: `src/lib.rs`

**Step 1: Write EventBus tests**

Create `src/infrastructure/mod.rs`:

```rust
pub mod event_bus;
pub use event_bus::EventBus;
```

Create `src/infrastructure/event_bus.rs`:

```rust
use anyhow::Result;

use crate::domain::YakEvent;
use crate::ports::{EventListener, EventStore};

pub struct EventBus {
    event_store: Box<dyn EventStore>,
    listeners: Vec<Box<dyn EventListener>>,
}

impl EventBus {
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            event_store,
            listeners: vec![],
        }
    }

    pub fn register(&mut self, listener: Box<dyn EventListener>) {
        self.listeners.push(listener);
    }

    pub fn publish(&mut self, event: YakEvent) -> Result<()> {
        // First: persist to event store (source of truth)
        self.event_store.append(&event)?;

        // Then: notify projections
        for listener in &mut self.listeners {
            listener.on_event(&event)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::InMemoryEventStore;

    struct TestListener {
        events: Vec<YakEvent>,
    }

    impl EventListener for TestListener {
        fn on_event(&mut self, event: &YakEvent) -> Result<()> {
            self.events.push(event.clone());
            Ok(())
        }
    }

    #[test]
    fn test_event_bus_publishes_to_store() {
        let store = InMemoryEventStore::new();
        let mut bus = EventBus::new(Box::new(store.clone()));

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        bus.publish(event.clone()).unwrap();

        let events = store.get_all_events().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn test_event_bus_notifies_listeners() {
        let store = InMemoryEventStore::new();
        let mut bus = EventBus::new(Box::new(store));

        let listener = TestListener { events: vec![] };
        bus.register(Box::new(listener));

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        bus.publish(event.clone()).unwrap();

        // Note: Can't easily test listener state after publish
        // due to ownership. Consider refactoring listener storage
        // or testing at integration level.
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib event_bus
```

Expected: Module not found

**Step 3: Add infrastructure module to lib**

Add to `src/lib.rs`:

```rust
pub mod infrastructure;
pub use infrastructure::EventBus;
```

**Step 4: Run tests**

```bash
cargo test --lib event_bus
```

Expected: Tests pass (note listener test needs improvement)

**Step 5: Commit**

```bash
git add src/infrastructure/ src/lib.rs
git commit -m "Add EventBus for event coordination

EventBus publishes events to EventStore and notifies
registered EventListener projections."
```

---

## Phase 3: Application Layer Updates

### Task 8: Update DirectoryStorage to Implement EventListener

**Files:**
- Modify: `src/adapters/storage/directory.rs`

**Step 1: Write test for EventListener implementation**

Add to `src/adapters/storage/directory.rs` tests:

```rust
#[test]
fn test_directory_storage_handles_added_event() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut storage = DirectoryStorage::new(temp_dir.path().to_str().unwrap());

    let event = YakEvent::Added {
        name: "test".to_string(),
    };

    storage.on_event(&event).unwrap();

    assert!(storage.yak_exists("test"));
}

#[test]
fn test_directory_storage_handles_context_updated_event() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut storage = DirectoryStorage::new(temp_dir.path().to_str().unwrap());

    // First add the yak
    storage.on_event(&YakEvent::Added {
        name: "test".to_string(),
    }).unwrap();

    // Then update context
    storage.on_event(&YakEvent::ContextUpdated {
        name: "test".to_string(),
        content: "new context".to_string(),
    }).unwrap();

    let yak = storage.get_yak("test").unwrap();
    assert_eq!(yak.context, Some("new context".to_string()));
}

#[test]
fn test_directory_storage_handles_state_updated_event() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut storage = DirectoryStorage::new(temp_dir.path().to_str().unwrap());

    storage.on_event(&YakEvent::Added {
        name: "test".to_string(),
    }).unwrap();

    storage.on_event(&YakEvent::StateUpdated {
        name: "test".to_string(),
        state: "wip".to_string(),
    }).unwrap();

    let yak = storage.get_yak("test").unwrap();
    assert_eq!(yak.state, "wip");
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib directory
```

Expected: Compilation error - EventListener not implemented

**Step 3: Implement EventListener for DirectoryStorage**

Add to `src/adapters/storage/directory.rs`:

```rust
use crate::ports::EventListener;
use crate::domain::YakEvent;

impl EventListener for DirectoryStorage {
    fn on_event(&mut self, event: &YakEvent) -> Result<()> {
        match event {
            YakEvent::Added { name } => {
                self.create_yak(name)?;
                // Set default state
                let state_path = self.yak_path(name).join("state");
                std::fs::write(state_path, "todo")?;
            }

            YakEvent::Removed { name } => {
                let yak_path = self.yak_path(name);
                if yak_path.exists() {
                    std::fs::remove_dir_all(yak_path)?;
                }
            }

            YakEvent::Moved { old_name, new_name } => {
                let old_path = self.yak_path(old_name);
                let new_path = self.yak_path(new_name);
                std::fs::rename(old_path, new_path)?;
            }

            YakEvent::ContextUpdated { name, content } => {
                self.write_field(name, "context", content)?;
            }

            YakEvent::StateUpdated { name, state } => {
                self.write_field(name, "state", state)?;
            }

            YakEvent::FieldUpdated {
                name,
                field_name,
                content,
            } => {
                self.write_field(name, field_name, content)?;
            }
        }
        Ok(())
    }
}
```

**Step 4: Run tests**

```bash
cargo test --lib directory
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/adapters/storage/directory.rs
git commit -m "Implement EventListener for DirectoryStorage

DirectoryStorage now acts as a projection, updating
.yaks/ files in response to domain events."
```

---

### Task 9: Implement Store for DirectoryStorage

**Files:**
- Modify: `src/adapters/storage/directory.rs`

**Step 1: Write test for Store implementation**

Add to `src/adapters/storage/directory.rs` tests:

```rust
#[test]
fn test_directory_storage_get_yak() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut storage = DirectoryStorage::new(temp_dir.path().to_str().unwrap());

    storage.on_event(&YakEvent::Added {
        name: "test".to_string(),
    }).unwrap();

    storage.on_event(&YakEvent::ContextUpdated {
        name: "test".to_string(),
        content: "context".to_string(),
    }).unwrap();

    let yak = storage.get_yak("test").unwrap();
    assert_eq!(yak.name, "test");
    assert_eq!(yak.state, "todo");
    assert_eq!(yak.context, Some("context".to_string()));
    assert!(yak.pending_events.is_empty());
}

#[test]
fn test_directory_storage_yak_exists() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut storage = DirectoryStorage::new(temp_dir.path().to_str().unwrap());

    storage.on_event(&YakEvent::Added {
        name: "test".to_string(),
    }).unwrap();

    assert!(storage.yak_exists("test"));
    assert!(!storage.yak_exists("missing"));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib directory
```

Expected: Compilation error - Store not implemented

**Step 3: Implement Store for DirectoryStorage**

Add to `src/adapters/storage/directory.rs`:

```rust
use crate::ports::Store;

impl Store for DirectoryStorage {
    fn get_yak(&self, name: &str) -> Result<Yak> {
        if !self.yak_exists(name) {
            return Err(anyhow::anyhow!("Yak '{}' not found", name));
        }

        let state = self.read_field(name, "state").unwrap_or_else(|_| "todo".to_string());
        let context = self.read_field(name, "context").ok();

        Ok(Yak {
            name: name.to_string(),
            state,
            context,
            pending_events: vec![],
        })
    }

    fn list_yaks(&self) -> Result<Vec<Yak>> {
        let path = std::path::Path::new(&self.yak_path);
        if !path.exists() {
            return Ok(vec![]);
        }

        let mut yaks = vec![];
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                yaks.push(self.get_yak(&name)?);
            }
        }

        Ok(yaks)
    }

    fn yak_exists(&self, name: &str) -> bool {
        self.yak_path(name).exists()
    }
}
```

**Step 4: Run tests**

```bash
cargo test --lib directory
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/adapters/storage/directory.rs
git commit -m "Implement Store for DirectoryStorage

DirectoryStorage now provides read model queries.
CQRS complete: EventListener (write) + Store (read)."
```

---

### Task 10: Update Application Struct

**Files:**
- Modify: `src/application/app.rs`

**Step 1: Write test for updated Application**

Add to `src/application/app.rs` tests (create tests module if needed):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{InMemoryDisplay, InMemoryInput, InMemoryStorage, InMemoryEventStore};
    use crate::infrastructure::EventBus;

    #[test]
    fn test_application_with_new_yak() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(&mut event_bus, &storage, &display, &input);

        app.with_new_yak("test", |yak| {
            assert_eq!(yak.name, "test");
            Ok(())
        }).unwrap();

        assert!(storage.yak_exists("test"));
    }

    #[test]
    fn test_application_with_yak() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(&mut event_bus, &storage, &display, &input);

        // Create yak first
        app.with_new_yak("test", |_| Ok(())).unwrap();

        // Now mutate it
        app.with_yak("test", |yak| {
            yak.update_state("wip".to_string())
        }).unwrap();

        let yak = storage.get_yak("test").unwrap();
        assert_eq!(yak.state, "wip");
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib app
```

Expected: Compilation errors - methods not found

**Step 3: Update Application struct**

Replace Application struct in `src/application/app.rs`:

```rust
use crate::infrastructure::EventBus;
use crate::ports::{DisplayPort, InputPort, Store};
use crate::domain::{validate_yak_name, Yak};
use anyhow::Result;

use super::UseCase;

pub struct Application<'a> {
    pub event_bus: &'a mut EventBus,
    pub store: &'a dyn Store,
    pub display: &'a dyn DisplayPort,
    pub input: &'a dyn InputPort,
}

impl<'a> Application<'a> {
    pub fn new(
        event_bus: &'a mut EventBus,
        store: &'a dyn Store,
        display: &'a dyn DisplayPort,
        input: &'a dyn InputPort,
    ) -> Self {
        Self {
            event_bus,
            store,
            display,
            input,
        }
    }

    pub fn with_yak<F>(&mut self, name: &str, f: F) -> Result<()>
    where
        F: FnOnce(&mut Yak) -> Result<()>,
    {
        let mut yak = self.store.get_yak(name)?;
        f(&mut yak)?;
        self.save(&mut yak)?;
        Ok(())
    }

    pub fn with_new_yak<F>(&mut self, name: &str, f: F) -> Result<()>
    where
        F: FnOnce(&mut Yak) -> Result<()>,
    {
        validate_yak_name(name)?;
        let mut yak = Yak::new(name.to_string());
        f(&mut yak)?;
        self.save(&mut yak)?;
        Ok(())
    }

    fn save(&mut self, aggregate: &mut Yak) -> Result<()> {
        for event in aggregate.take_events() {
            self.event_bus.publish(event)?;
        }
        Ok(())
    }

    pub fn handle<U: UseCase>(&mut self, use_case: U) -> Result<()> {
        use_case.execute(self)
    }
}
```

**Step 4: Run tests**

```bash
cargo test --lib app
```

Expected: Compilation errors in other files that use Application

**Step 5: Don't commit yet - need to fix use cases first**

---

## Phase 4: Refactor Use Cases

### Task 11: Refactor AddYak Use Case

**Files:**
- Modify: `src/application/add_yak.rs`

**Step 1: Write test for refactored AddYak**

Update tests in `src/application/add_yak.rs`:

```rust
#[test]
fn test_add_yak_creates_yak() {
    let mut event_store = InMemoryEventStore::new();
    let mut event_bus = EventBus::new(Box::new(event_store));

    let storage = InMemoryStorage::new();
    event_bus.register(Box::new(storage.clone()));

    let display = InMemoryDisplay::new();
    let input = InMemoryInput::new();
    let mut app = Application::new(&mut event_bus, &storage, &display, &input);

    let use_case = AddYak::new("test-yak");
    use_case.execute(&mut app).unwrap();

    assert!(storage.yak_exists("test-yak"));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib add_yak
```

Expected: Compilation errors

**Step 3: Refactor AddYak to use with_new_yak**

Replace execute method in `src/application/add_yak.rs`:

```rust
impl AddYak {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        app.with_new_yak(&self.name, |yak| {
            // Generate template
            let template = self.generate_context_template()?;

            // Request content via input port
            if let Some(content) = app.input.request_content(None, Some(&template))? {
                if !content.trim().is_empty() {
                    yak.update_context(content)?;
                }
            }

            Ok(())
        })
    }

    // Keep generate_context_template method unchanged
}
```

**Step 4: Update UseCase trait implementation**

Update UseCase impl:

```rust
impl UseCase for AddYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

**Step 5: Run tests**

```bash
cargo test --lib add_yak
```

Expected: All tests pass

**Step 6: Commit**

```bash
git add src/application/add_yak.rs
git commit -m "Refactor AddYak to use event-sourced pattern

Use Application.with_new_yak() closure API.
Yak aggregate emits Added and ContextUpdated events."
```

---

### Task 12: Refactor DoneYak Use Case

**Files:**
- Modify: `src/application/done_yak.rs`

**Step 1: Update tests**

```rust
#[test]
fn test_done_yak_marks_yak_done() {
    let mut event_store = InMemoryEventStore::new();
    let mut event_bus = EventBus::new(Box::new(event_store));

    let storage = InMemoryStorage::new();
    event_bus.register(Box::new(storage.clone()));

    let display = InMemoryDisplay::new();
    let input = InMemoryInput::new();
    let mut app = Application::new(&mut event_bus, &storage, &display, &input);

    // Create yak first
    app.with_new_yak("test", |_| Ok(())).unwrap();

    // Mark done
    let use_case = DoneYak::new("test");
    use_case.execute(&mut app).unwrap();

    let yak = storage.get_yak("test").unwrap();
    assert!(yak.is_done());
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib done_yak
```

Expected: Compilation error

**Step 3: Refactor DoneYak**

Replace in `src/application/done_yak.rs`:

```rust
impl DoneYak {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        app.with_yak(&self.name, |yak| yak.update_state("done".to_string()))
    }
}

impl UseCase for DoneYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

**Step 4: Run tests**

```bash
cargo test --lib done_yak
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/application/done_yak.rs
git commit -m "Refactor DoneYak to use event-sourced pattern

Use Application.with_yak() closure API.
Sets state to \"done\" which emits StateUpdated event."
```

---

### Task 13: Refactor SetState Use Case

**Files:**
- Modify: `src/application/set_state.rs`

**Step 1: Refactor SetState**

```rust
impl SetState {
    pub fn new(name: &str, state: &str) -> Self {
        Self {
            name: name.to_string(),
            state: state.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        app.with_yak(&self.name, |yak| yak.update_state(self.state.clone()))
    }
}

impl UseCase for SetState {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

**Step 2: Run tests**

```bash
cargo test --lib set_state
```

Expected: All tests pass

**Step 3: Commit**

```bash
git add src/application/set_state.rs
git commit -m "Refactor SetState to use event-sourced pattern"
```

---

### Task 14: Refactor EditContext Use Case

**Files:**
- Modify: `src/application/edit_context.rs`

**Step 1: Refactor EditContext**

```rust
impl EditContext {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Get current context
        let current_context = app
            .store
            .get_yak(&self.name)?
            .context
            .unwrap_or_default();

        // Request new content via input
        if let Some(content) = app.input.request_content(Some(&current_context), None)? {
            app.with_yak(&self.name, |yak| yak.update_context(content))?;
        }

        Ok(())
    }
}

impl UseCase for EditContext {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

**Step 2: Run tests**

```bash
cargo test --lib edit_context
```

Expected: All tests pass

**Step 3: Commit**

```bash
git add src/application/edit_context.rs
git commit -m "Refactor EditContext to use event-sourced pattern"
```

---

### Task 15: Refactor RemoveYak Use Case

**Files:**
- Modify: `src/application/remove_yak.rs`

**Step 1: Refactor RemoveYak**

```rust
impl RemoveYak {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Verify yak exists first
        let mut yak = app.store.get_yak(&self.name)?;

        // Emit Removed event
        yak.pending_events.push(YakEvent::Removed {
            name: yak.name.clone(),
        });

        app.save(&mut yak)?;
        Ok(())
    }
}

impl UseCase for RemoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

Note: This is a bit awkward - we load the yak just to emit a Removed event. Consider adding a helper method or accepting this pattern.

**Step 2: Run tests**

```bash
cargo test --lib remove_yak
```

Expected: All tests pass

**Step 3: Commit**

```bash
git add src/application/remove_yak.rs
git commit -m "Refactor RemoveYak to use event-sourced pattern"
```

---

### Task 16: Refactor PruneYaks Use Case

**Files:**
- Modify: `src/application/prune_yaks.rs`

**Step 1: Refactor PruneYaks**

```rust
impl PruneYaks {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let yaks = app.store.list_yaks()?;

        for yak in yaks.iter().filter(|y| y.is_done()) {
            let mut yak_to_remove = yak.clone();
            yak_to_remove.pending_events.push(YakEvent::Removed {
                name: yak_to_remove.name.clone(),
            });
            app.save(&mut yak_to_remove)?;
        }

        Ok(())
    }
}

impl UseCase for PruneYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

**Step 2: Run tests**

```bash
cargo test --lib prune_yaks
```

Expected: All tests pass

**Step 3: Commit**

```bash
git add src/application/prune_yaks.rs
git commit -m "Refactor PruneYaks to use event-sourced pattern

Emits multiple Removed events, one per done yak."
```

---

### Task 17: Refactor Remaining Use Cases

**Files:**
- Modify: `src/application/show_context.rs`
- Modify: `src/application/show_field.rs`
- Modify: `src/application/write_field.rs`
- Modify: `src/application/move_yak.rs`
- Modify: `src/application/list_yaks.rs`

Follow the same pattern for each:
1. Update to use Application's new signature
2. Read-only use cases (show_context, show_field, list_yaks) only need store access
3. Write use cases (write_field, move_yak) need event emission

**Step 1: Update ShowContext (read-only)**

```rust
impl ShowContext {
    pub fn execute(&self, app: &Application) -> Result<()> {
        let yak = app.store.get_yak(&self.name)?;

        if let Some(context) = &yak.context {
            app.display.info(context);
        }

        Ok(())
    }
}
```

**Step 2: Update ShowField (read-only)**

Similar to ShowContext - just query store.

**Step 3: Update ListYaks (read-only)**

```rust
impl ListYaks {
    pub fn execute(&self, app: &Application) -> Result<()> {
        let yaks = app.store.list_yaks()?;

        // Format and display (existing logic)

        Ok(())
    }
}
```

**Step 4: Update WriteField**

```rust
impl WriteField {
    pub fn execute(&self, app: &mut Application) -> Result<()> {
        app.with_yak(&self.name, |yak| {
            yak.update_field(self.field_name.clone(), self.content.clone())
        })
    }
}
```

Note: Need to add `update_field()` method to Yak aggregate.

**Step 5: Update MoveYak**

```rust
impl MoveYak {
    pub fn execute(&self, app: &mut Application) -> Result<()> {
        app.with_yak(&self.from, |yak| {
            yak.move_to(self.to.clone())
        })
    }
}
```

Note: Need to add `move_to()` method to Yak aggregate.

**Step 6: Add missing methods to Yak**

Add to `src/domain/yak.rs`:

```rust
impl Yak {
    pub fn move_to(&mut self, new_name: String) -> Result<()> {
        validate_yak_name(&new_name)?;

        let old_name = self.name.clone();
        self.name = new_name.clone();

        self.pending_events.push(YakEvent::Moved {
            old_name,
            new_name,
        });
        Ok(())
    }

    pub fn update_field(&mut self, field_name: String, content: String) -> Result<()> {
        self.pending_events.push(YakEvent::FieldUpdated {
            name: self.name.clone(),
            field_name,
            content,
        });
        Ok(())
    }
}
```

**Step 7: Run all tests**

```bash
cargo test --lib
```

Expected: All tests pass

**Step 8: Commit**

```bash
git add src/application/*.rs src/domain/yak.rs
git commit -m "Refactor remaining use cases to event-sourced pattern

- Update read-only use cases to use store
- Add move_to() and update_field() to Yak aggregate
- All use cases now follow event-sourced pattern"
```

---

### Task 18: Update main.rs to Wire Up EventBus

**Files:**
- Modify: `src/main.rs`

**Step 1: Update main.rs initialization**

Replace Application initialization in `src/main.rs`:

```rust
use yx::infrastructure::EventBus;
use yx::adapters::InMemoryEventStore;

fn main() -> Result<()> {
    // ... existing setup ...

    // Create event infrastructure
    let mut event_store = InMemoryEventStore::new();
    let mut event_bus = EventBus::new(Box::new(event_store));

    // Create storage
    let mut storage = DirectoryStorage::new(&yak_path);

    // Register storage as projection
    event_bus.register(Box::new(storage.clone()));

    // Create other adapters
    let display = ConsoleDisplay::new();
    let log = GitLog::new();
    let input = ConsoleInput::new();

    // Create application
    let mut app = Application::new(&mut event_bus, &storage, &display, &input);

    // ... existing command handling ...
}
```

Note: This requires making DirectoryStorage cloneable or using Arc<Mutex<>>. Consider refactoring storage to be thread-safe.

**Step 2: Handle mutability**

Application now needs `&mut self` for write operations. Update command handler to pass mutable reference.

**Step 3: Run binary**

```bash
cargo build --release
./target/release/yx add "test yak"
./target/release/yx ls
```

Expected: Commands work as before

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "Wire up EventBus in main

Initialize EventBus with InMemoryEventStore and register
DirectoryStorage as projection."
```

---

### Task 19: Run Full Test Suite

**Files:**
- None (just running tests)

**Step 1: Run unit tests**

```bash
cargo test --lib
```

Expected: All unit tests pass

**Step 2: Run Cucumber tests**

```bash
cargo test --test cucumber
```

Expected: All Cucumber tests pass in both modes

**Step 3: Run shellspec tests**

```bash
shellspec
```

Expected: All shellspec tests pass

**Step 4: Run dev check**

```bash
dev check
```

Expected: All checks pass (tests + lint + audit)

**Step 5: If any tests fail, fix them before proceeding**

Common issues:
- Storage not implementing Clone
- Application mutability
- Event handling in projections

Fix incrementally and commit fixes.

---

### Task 20: Update Documentation

**Files:**
- Modify: `docs/plans/2026-02-10-event-sourced-architecture-design.md`

**Step 1: Mark design as implemented**

Update header:

```markdown
**Status:** ~~Draft~~ Implemented
```

**Step 2: Add implementation notes section**

Add at end of design doc:

```markdown
## Implementation Notes

**Completed:** 2026-02-10

**Changes from design:**
- [List any deviations from original design]
- [Document any issues encountered]
- [Note any remaining work]

**Verification:**
- All unit tests passing
- All Cucumber tests passing
- All shellspec tests passing
- dev check passes

**Next steps:**
- Implement GitEventStore (future)
- Remove shellspec tests once Cucumber conversion complete
- Consider refactoring storage thread-safety
```

**Step 3: Commit**

```bash
git add docs/plans/2026-02-10-event-sourced-architecture-design.md
git commit -m "Mark event-sourced architecture as implemented"
```

---

## Final Step: Merge to Main

**Step 1: Ensure all tests pass**

```bash
dev check
```

**Step 2: Review git log**

```bash
git log --oneline
```

Verify commits are clean and incremental.

**Step 3: Return to main and merge**

```bash
cd /Users/mattwynne/git/mattwynne/yaks  # Return to main repo
git merge --no-ff event-sourced-architecture -m "Merge event-sourced-architecture: Event sourcing with CQRS

Complete refactoring to event-sourced architecture:
- Events are source of truth (YakEvent enum)
- Storage acts as event projection
- Rich domain aggregates emit events
- Application provides closure-based API
- CQRS: separate read (Store) and write (EventListener) models

All tests passing. Ready for GitEventStore implementation."
```

**Step 4: Clean up worktree**

```bash
git worktree remove .worktrees/event-sourced-architecture
git branch -d event-sourced-architecture
```

**Step 5: Mark yak done**

```bash
yx done "Refactor until exemplary"
```

---

## Summary

This plan refactors yak to use event sourcing with CQRS and DDD patterns:

**What we built:**
- ✅ YakEvent domain events (Added, Removed, ContextUpdated, etc.)
- ✅ EventBus for event coordination
- ✅ EventStore trait with InMemoryEventStore
- ✅ EventListener trait for projections
- ✅ Store trait for read model
- ✅ DirectoryStorage implements both EventListener and Store (CQRS)
- ✅ Rich Yak aggregate that emits events
- ✅ Application with closure-based API (with_yak, with_new_yak)
- ✅ All use cases refactored to event-sourced pattern

**Testing approach:**
- TDD: Write test → Run (fail) → Implement → Run (pass) → Commit
- Unit tests for each component
- Integration tests for use cases
- Cucumber/shellspec tests verify behavior unchanged

**Benefits achieved:**
- Business logic in domain (Yak aggregate)
- Complete audit trail (all events stored)
- Clean use cases (closure-based API)
- Extensibility (easy to add projections)
- CQRS (separate read/write models)

**Future work:**
- Implement GitEventStore for persistent event storage
- Event replay and time travel
- Additional projections (analytics, notifications)
- Event versioning strategy
