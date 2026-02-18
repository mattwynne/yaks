# Author and Timestamp on Events - Implementation Plan

> **For Claude:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add author and timestamp metadata to every domain event, persist it through round-trips, and display it in `yx log`.

**Architecture:** EventMetadata (Author + Timestamp) lives in the domain layer. Every YakEvent variant carries metadata. YakMap receives metadata at construction time and stamps it on every event it creates. A new AuthenticationPort provides current-user identity. The git adapter writes/reads metadata as commit signatures. Disk projection writes `.metadata.json` for created_by/created_at round-trip.

**Tech Stack:** Rust, git2 (for commit signatures), serde_json (for .metadata.json), chrono (for timestamp formatting in yx log)

**Spec:** `docs/superpowers/specs/2026-02-18-author-timestamp-on-events-design.md`

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src/domain/event_metadata.rs` | Author, Timestamp, EventMetadata types |
| Create | `src/domain/ports/authentication.rs` | AuthenticationPort trait |
| Create | `src/adapters/authentication/mod.rs` | Module declaration |
| Create | `src/adapters/authentication/git.rs` | GitAuthentication adapter |
| Modify | `src/domain/mod.rs` | Re-export new types |
| Modify | `src/domain/ports/mod.rs` | Re-export AuthenticationPort |
| Modify | `src/adapters/mod.rs` | Register authentication module |
| Modify | `src/domain/event.rs` | Add metadata to YakEvent |
| Modify | `src/domain/yak_map.rs` | Accept EventMetadata, stamp on events |
| Modify | `src/domain/yak.rs` | Add created_by, created_at fields |
| Modify | `src/domain/field.rs` | Add `.metadata.json` to RESERVED_FIELDS |
| Modify | `src/application/app.rs` | Accept AuthenticationPort, construct metadata |
| Modify | `src/application/add_yak.rs` | Add with_author/with_timestamp builders |
| Modify | `src/adapters/event_store/git.rs` | Use metadata for signatures, extract on read |
| Modify | `src/adapters/yak_store/directory.rs` | Read .metadata.json, populate Yak fields |
| Modify | `src/projections/write_to_yak_store.rs` | Write .metadata.json on Added event (via write_field) |
| Modify | `src/application/show_log.rs` | Display author + timestamp via DisplayPort |
| Modify | `src/domain/ports/user_display.rs` | Add log_entry method to DisplayPort |
| Modify | `src/main.rs` | Wire AuthenticationPort, pass metadata, update hard reset |
| Modify | `src/adapters/yak_store/in_memory.rs` | Populate Yak.created_by/created_at defaults in tests |
| Modify | `features/log.feature` | Add scenario for author/timestamp display |
| Modify | `features/reset.feature` | Add scenario for author preservation through hard reset |
| Modify | `tests/features/steps.rs` | Add step defs for new scenarios |

**New dependencies (add to `[dependencies]` in Cargo.toml):**
- `serde_json = "1"` -- for .metadata.json serialization
- `chrono = "0.4"` -- for timestamp formatting in yx log

---

## Chunk 1: Domain Types and AuthenticationPort

### Task 1: Create EventMetadata domain types

**Files:**
- Create: `src/domain/event_metadata.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Create event_metadata.rs with Author, Timestamp, EventMetadata**

```rust
// src/domain/event_metadata.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub email: String,
}

impl Author {
    pub fn unknown() -> Self {
        Self {
            name: "unknown".to_string(),
            email: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        )
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn as_epoch_secs(&self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMetadata {
    pub author: Author,
    pub timestamp: Timestamp,
}

impl EventMetadata {
    pub fn new(author: Author, timestamp: Timestamp) -> Self {
        Self { author, timestamp }
    }

    pub fn default_legacy() -> Self {
        Self {
            author: Author::unknown(),
            timestamp: Timestamp::zero(),
        }
    }
}
```

- [ ] **Step 2: Register module and re-export in domain/mod.rs**

Add to `src/domain/mod.rs`:
```rust
pub mod event_metadata;
pub use event_metadata::{Author, EventMetadata, Timestamp};
```

- [ ] **Step 3: Run tests to verify no regressions**

Run: `cargo test --lib`
Expected: All existing tests pass, no compilation errors.

- [ ] **Step 4: Commit**

```bash
git mit me && git add src/domain/event_metadata.rs src/domain/mod.rs && git commit -m "Add EventMetadata domain types"
```

### Task 2: Create AuthenticationPort and GitAuthentication adapter

**Files:**
- Create: `src/domain/ports/authentication.rs`
- Modify: `src/domain/ports/mod.rs`
- Create: `src/adapters/authentication/mod.rs`
- Create: `src/adapters/authentication/git.rs`
- Modify: `src/adapters/mod.rs`

- [ ] **Step 1: Write unit test for GitAuthentication**

In `src/adapters/authentication/git.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn reads_author_from_git_config() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        let auth = GitAuthentication::new(tmp.path()).unwrap();
        let author = auth.current_author();

        assert_eq!(author.name, "Test User");
        assert_eq!(author.email, "test@example.com");
    }

    #[test]
    fn falls_back_when_git_config_missing() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();

        let auth = GitAuthentication::new(tmp.path()).unwrap();
        let author = auth.current_author();

        // Should not panic, should return some default
        assert!(!author.name.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib authentication`
Expected: FAIL - module doesn't exist yet.

- [ ] **Step 3: Create AuthenticationPort trait**

`src/domain/ports/authentication.rs`:
```rust
use crate::domain::event_metadata::Author;

pub trait AuthenticationPort {
    fn current_author(&self) -> Author;
}
```

Add to `src/domain/ports/mod.rs`:
```rust
pub mod authentication;
pub use authentication::AuthenticationPort;
```

- [ ] **Step 4: Create GitAuthentication adapter**

`src/adapters/authentication/mod.rs`:
```rust
pub mod git;
pub use git::GitAuthentication;
```

`src/adapters/authentication/git.rs`:
```rust
use std::path::Path;

use anyhow::Result;

use crate::domain::event_metadata::Author;
use crate::domain::ports::AuthenticationPort;

pub struct GitAuthentication {
    name: String,
    email: String,
}

impl GitAuthentication {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo = git2::Repository::open(repo_path)?;
        let config = repo.config()?;
        let name = config
            .get_string("user.name")
            .unwrap_or_else(|_| "yx".to_string());
        let email = config
            .get_string("user.email")
            .unwrap_or_else(|_| "yx@localhost".to_string());
        Ok(Self { name, email })
    }
}

impl AuthenticationPort for GitAuthentication {
    fn current_author(&self) -> Author {
        Author {
            name: self.name.clone(),
            email: self.email.clone(),
        }
    }
}
```

Add to `src/adapters/mod.rs`:
```rust
pub mod authentication;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib authentication`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git mit me && git add src/domain/ports/authentication.rs src/domain/ports/mod.rs src/adapters/authentication/ src/adapters/mod.rs && git commit -m "Add AuthenticationPort and GitAuthentication"
```

### Task 3: Add metadata to YakEvent

**Files:**
- Modify: `src/domain/event.rs`

This is a large cross-cutting change. Every YakEvent now carries metadata. The approach: add a `metadata` field to each variant's inner struct... No - per the design, `EventMetadata` is on the `YakEvent` enum itself, not on the inner event structs. We'll restructure YakEvent to pair each variant with metadata.

- [ ] **Step 1: Write test for metadata accessor**

Add test in `src/domain/event.rs`:
```rust
#[test]
fn metadata_returns_event_metadata() {
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

    let metadata = EventMetadata::new(
        Author { name: "Matt".to_string(), email: "matt@example.com".to_string() },
        Timestamp(1708300800),
    );
    let event = YakEvent::Added(
        AddedEvent {
            name: Name::from("test"),
            id: YakId::from("test-a1b2"),
            parent_id: None,
        },
        metadata.clone(),
    );
    assert_eq!(event.metadata(), &metadata);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib event::tests::metadata_returns_event_metadata`
Expected: FAIL - YakEvent::Added takes 1 arg, not 2.

- [ ] **Step 3: Restructure YakEvent to carry metadata**

Change `src/domain/event.rs`:
```rust
use super::event_metadata::EventMetadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YakEvent {
    Added(AddedEvent, EventMetadata),
    Removed(RemovedEvent, EventMetadata),
    Moved(MovedEvent, EventMetadata),
    FieldUpdated(FieldUpdatedEvent, EventMetadata),
}

impl YakEvent {
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            Self::Added(_, m) => m,
            Self::Removed(_, m) => m,
            Self::Moved(_, m) => m,
            Self::FieldUpdated(_, m) => m,
        }
    }
```

Update all match arms in `format_message`, `parse`, and `yak_id` to handle the new tuple shape. For `parse`, use `EventMetadata::default_legacy()` since parsed events from commit messages don't carry metadata inline (it comes from git commit info separately).

This will cause compilation errors across the codebase. Fix each call site:
- `yak_map.rs`: All `.push(YakEvent::Variant(..))` calls need metadata
- `write_to_yak_store.rs`: All `match event` arms need to destructure the metadata
- `event_bus.rs`: No change (just passes events through)
- `git.rs` event store: `append` and `get_all_events` need adjustment
- `show_log.rs`: match arms
- `main.rs`: any direct event construction
- All test files that construct YakEvent

**This is the largest mechanical change.** Fix compilation errors one file at a time, using `EventMetadata::default_legacy()` for all existing code paths that don't have real metadata yet. We'll wire real metadata in later tasks.

- [ ] **Step 4: Fix all compilation errors across the codebase**

The key pattern: everywhere that creates a `YakEvent`, add `EventMetadata::default_legacy()` as the second tuple element. Everywhere that matches on a `YakEvent`, add `_` or a binding for the metadata field.

Key files to update (in suggested order):
1. `src/domain/event.rs` - the enum itself and its methods
2. `src/domain/yak_map.rs` - all `.push(YakEvent::...)` calls
3. `src/projections/write_to_yak_store.rs` - all match arms
4. `src/adapters/event_store/git.rs` - parse returns, append, snapshot_events
5. `src/application/show_log.rs` - match in execute
6. `src/main.rs` - any direct event handling
7. All `#[cfg(test)]` blocks in the above files

- [ ] **Step 5: Run full test suite**

Run: `cargo test --lib && cargo test --test cucumber --features test-support`
Expected: All tests pass. Metadata is default_legacy() everywhere for now.

- [ ] **Step 6: Commit**

```bash
git mit me && git add -A && git commit -m "Add EventMetadata to every YakEvent variant"
```

### Task 4: YakMap accepts and stamps metadata

**Files:**
- Modify: `src/domain/yak_map.rs`

- [ ] **Step 1: Write test for YakMap stamping metadata**

Add test in `src/domain/yak_map.rs`:
```rust
#[test]
fn test_add_yak_stamps_provided_metadata() {
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

    let metadata = EventMetadata::new(
        Author { name: "Matt".to_string(), email: "matt@example.com".to_string() },
        Timestamp(1708300800),
    );
    let mut map = YakMap::with_metadata(metadata.clone());
    map.add_yak("test", None, None, None, None, vec![]).unwrap();
    let events = map.take_events();

    assert_eq!(events[0].metadata(), &metadata);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib yak_map::tests::test_add_yak_stamps_provided_metadata`
Expected: FAIL - `YakMap::with_metadata` doesn't exist.

- [ ] **Step 3: Add metadata field to YakMap**

In `src/domain/yak_map.rs`:
- Add `metadata: EventMetadata` field to `YakMap` struct
- Add `pub fn with_metadata(metadata: EventMetadata) -> Self` constructor
- Update `new()` (test-only) to use `EventMetadata::default_legacy()`
- Update `from_store()` to accept metadata parameter
- Change all `.push(YakEvent::Variant(inner))` to `.push(YakEvent::Variant(inner, self.metadata.clone()))`

- [ ] **Step 4: Update Application to pass metadata to YakMap**

In `src/application/app.rs`:
- `with_yak_map` and `with_yak_map_result` currently call `YakMap::from_store(self.store)`.
- Change to `YakMap::from_store(self.store, metadata)` where metadata is constructed from defaults for now (we'll wire AuthenticationPort later).
- Add a `metadata: EventMetadata` field to Application or construct it in `with_yak_map*` methods.

- [ ] **Step 5: Run full test suite**

Run: `cargo test --lib && cargo test --test cucumber --features test-support`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git mit me && git add src/domain/yak_map.rs src/application/app.rs && git commit -m "YakMap stamps EventMetadata on all events"
```

## Chunk 2: Application Layer Wiring

### Task 5: Wire AuthenticationPort into Application

**Files:**
- Modify: `src/application/app.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write test for Application using AuthenticationPort**

Add test in `src/application/app.rs`:
```rust
#[test]
fn test_application_stamps_author_on_events() {
    use crate::domain::event_metadata::Author;
    use crate::domain::ports::AuthenticationPort;

    struct TestAuth;
    impl AuthenticationPort for TestAuth {
        fn current_author(&self) -> Author {
            Author { name: "Test".to_string(), email: "test@test.com".to_string() }
        }
    }

    let event_store = InMemoryEventStore::new();
    let mut event_bus = EventBus::new(Box::new(event_store.clone()));
    let storage = InMemoryStorage::new();
    event_bus.register(Box::new(storage.clone()));
    let display = InMemoryDisplay::new();
    let input = InMemoryInput::new();
    let auth = TestAuth;

    let mut app = Application::new(
        &mut event_bus, &storage, &display, &input, None, None, &auth,
    );

    app.with_yak_map(|yak_map| {
        yak_map.add_yak("test", None, None, None, None, vec![])?;
        Ok(())
    }).unwrap();

    let events = event_store.get_all_events().unwrap();
    assert_eq!(events[0].metadata().author.name, "Test");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib app::tests::test_application_stamps_author_on_events`
Expected: FAIL - Application::new doesn't accept auth parameter.

- [ ] **Step 3: Add AuthenticationPort to Application**

In `src/application/app.rs`:
- Add `auth: &'a dyn AuthenticationPort` field
- Update `new()` to accept it
- In `with_yak_map` and `with_yak_map_result`, construct `EventMetadata` from `self.auth.current_author()` and `Timestamp::now()`, pass to `YakMap::from_store()`

- [ ] **Step 4: Fix all Application::new() call sites**

Update every `Application::new(...)` call to pass an `&dyn AuthenticationPort`:
- `src/main.rs`: Use `GitAuthentication` when in a git repo, or a default fallback
- All test files: Create a simple test auth adapter

For tests, create `InMemoryAuthentication` in the test-support adapters:
```rust
#[cfg(any(test, feature = "test-support"))]
pub struct InMemoryAuthentication;

#[cfg(any(test, feature = "test-support"))]
impl AuthenticationPort for InMemoryAuthentication {
    fn current_author(&self) -> Author {
        Author { name: "test".to_string(), email: "test@test.com".to_string() }
    }
}
```

- [ ] **Step 5: Update main.rs to wire GitAuthentication**

```rust
let auth = if let Some(ref root) = repo_root {
    yx::adapters::authentication::GitAuthentication::new(root)?
} else {
    // Fallback for no-git mode
    // Create a simple default auth
};
```

Pass `&auth` to `Application::new(...)`.

- [ ] **Step 6: Run full test suite**

Run: `cargo test --lib && cargo test --test cucumber --features test-support`
Expected: All tests pass. Events now carry real author metadata.

- [ ] **Step 7: Commit**

```bash
git mit me && git add src/application/app.rs src/main.rs src/adapters/ && git commit -m "Wire AuthenticationPort into Application"
```

### Task 6: AddYak with_author and with_timestamp overrides

**Files:**
- Modify: `src/application/add_yak.rs`
- Modify: `src/application/app.rs`

- [ ] **Step 1: Write test for AddYak with_author override**

Add test in `src/application/add_yak.rs`:
```rust
#[test]
fn test_add_yak_with_author_override() {
    use crate::domain::event_metadata::{Author, Timestamp};

    let event_store = InMemoryEventStore::new();
    let mut event_bus = EventBus::new(Box::new(event_store.clone()));
    let storage = InMemoryStorage::new();
    event_bus.register(Box::new(storage.clone()));
    let display = InMemoryDisplay::new();
    let input = InMemoryInput::new();
    let auth = InMemoryAuthentication;

    let mut app = Application::new(
        &mut event_bus, &storage, &display, &input, None, None, &auth,
    );

    let custom_author = Author {
        name: "Original Author".to_string(),
        email: "original@example.com".to_string(),
    };

    AddYak::new("test")
        .with_author(Some(custom_author.clone()))
        .with_timestamp(Some(Timestamp(1708300800)))
        .execute(&mut app)
        .unwrap();

    let events = event_store.get_all_events().unwrap();
    assert_eq!(events[0].metadata().author, custom_author);
    assert_eq!(events[0].metadata().timestamp, Timestamp(1708300800));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib add_yak::tests::test_add_yak_with_author_override`
Expected: FAIL - `with_author` and `with_timestamp` methods don't exist.

- [ ] **Step 3: Add builder methods to AddYak**

In `src/application/add_yak.rs`:
```rust
use crate::domain::event_metadata::{Author, Timestamp};

pub struct AddYak {
    // ... existing fields ...
    author_override: Option<Author>,
    timestamp_override: Option<Timestamp>,
}

impl AddYak {
    // ... existing methods ...

    pub fn with_author(mut self, author: Option<Author>) -> Self {
        self.author_override = author;
        self
    }

    pub fn with_timestamp(mut self, timestamp: Option<Timestamp>) -> Self {
        self.timestamp_override = timestamp;
        self
    }
}
```

- [ ] **Step 4: Application uses overrides when executing AddYak**

The Application needs to know about the overrides when constructing the YakMap metadata. Two approaches:

**Approach:** AddYak's `execute()` method constructs custom metadata if overrides are set, and passes it to the Application's `with_yak_map_result` via a new method that accepts explicit metadata. Add `with_yak_map_result_and_metadata` or make `with_yak_map_result` accept an optional metadata override.

Simpler: Add a method on Application:
```rust
pub fn with_yak_map_result_using_metadata<T, F>(
    &mut self,
    metadata: EventMetadata,
    f: F,
) -> Result<T>
where
    F: FnOnce(&mut YakMap) -> Result<T>,
{
    let mut yak_map = YakMap::from_store(self.store, metadata)?;
    let result = f(&mut yak_map)?;
    self.save_yak_map(&mut yak_map)?;
    Ok(result)
}
```

In `AddYak::execute()`:
```rust
let metadata = EventMetadata::new(
    self.author_override.clone().unwrap_or_else(|| app.current_author()),
    self.timestamp_override.unwrap_or_else(Timestamp::now),
);

let id = app.with_yak_map_result_using_metadata(metadata, |yak_map| {
    yak_map.add_yak(...)
})?;
```

Add `current_author()` helper on Application that delegates to `self.auth`.

- [ ] **Step 5: Run full test suite**

Run: `cargo test --lib && cargo test --test cucumber --features test-support`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git mit me && git add src/application/add_yak.rs src/application/app.rs && git commit -m "Add with_author/with_timestamp to AddYak"
```

## Chunk 3: Git Adapter and Disk Persistence

### Task 7: Git adapter writes metadata as commit signatures

**Files:**
- Modify: `src/adapters/event_store/git.rs`

- [ ] **Step 1: Write test for commit signature from metadata**

Add test in `src/adapters/event_store/git.rs`:
```rust
#[test]
fn append_uses_event_metadata_for_commit_signature() {
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

    let (_tmp, mut store) = setup_test_repo();

    let metadata = EventMetadata::new(
        Author { name: "Custom Author".to_string(), email: "custom@example.com".to_string() },
        Timestamp(1708300800),
    );

    store.append(&YakEvent::Added(
        AddedEvent {
            name: Name::from("test"),
            id: YakId::from("test-a1b2"),
            parent_id: None,
        },
        metadata,
    )).unwrap();

    let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
    let commit = store.repo.find_commit(oid).unwrap();
    assert_eq!(commit.author().name().unwrap(), "Custom Author");
    assert_eq!(commit.author().email().unwrap(), "custom@example.com");
    assert_eq!(commit.author().when().seconds(), 1708300800);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib git::tests::append_uses_event_metadata`
Expected: FAIL - commit uses repo.signature(), not event metadata.

- [ ] **Step 3: Implement: use event metadata for git signatures**

In `append()` method, replace:
```rust
let sig = self.repo.signature()
    .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;
```

With:
```rust
let meta = event.metadata();
let time = git2::Time::new(meta.timestamp.as_epoch_secs(), 0);
let sig = git2::Signature::new(&meta.author.name, &meta.author.email, &time)?;
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib git::tests`
Expected: All git event store tests pass.

- [ ] **Step 5: Commit**

```bash
git mit me && git add src/adapters/event_store/git.rs && git commit -m "Git adapter uses event metadata for signatures"
```

### Task 8: Git adapter reads metadata from commits

**Files:**
- Modify: `src/adapters/event_store/git.rs`

- [ ] **Step 1: Write test for reading metadata from commits**

Add test in `src/adapters/event_store/git.rs`:
```rust
#[test]
fn get_all_events_populates_metadata_from_commits() {
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

    let (_tmp, mut store) = setup_test_repo();

    let metadata = EventMetadata::new(
        Author { name: "Reader Test".to_string(), email: "reader@test.com".to_string() },
        Timestamp(1708300800),
    );

    store.append(&YakEvent::Added(
        AddedEvent {
            name: Name::from("test"),
            id: YakId::from("test-a1b2"),
            parent_id: None,
        },
        metadata.clone(),
    )).unwrap();

    let events = EventStore::get_all_events(&store).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].metadata().author.name, "Reader Test");
    assert_eq!(events[0].metadata().author.email, "reader@test.com");
    assert_eq!(events[0].metadata().timestamp, Timestamp(1708300800));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib git::tests::get_all_events_populates_metadata`
Expected: FAIL - get_all_events uses `EventMetadata::default_legacy()` from parse.

- [ ] **Step 3: Update get_all_events to extract commit metadata**

In `get_all_events()`, after parsing the event from the commit message, replace the event's metadata with data extracted from the commit:

```rust
let commit = self.repo.find_commit(oid)?;
let message = commit.message().unwrap_or("").trim();
// ...
match YakEvent::parse(message) {
    Ok(event) => {
        let author = Author {
            name: commit.author().name().unwrap_or("unknown").to_string(),
            email: commit.author().email().unwrap_or("").to_string(),
        };
        let timestamp = Timestamp(commit.author().when().seconds());
        let metadata = EventMetadata::new(author, timestamp);
        events.push(event.with_metadata(metadata));
    }
    Err(_) => continue,
}
```

Add a `with_metadata` method on YakEvent:
```rust
pub fn with_metadata(self, metadata: EventMetadata) -> Self {
    match self {
        Self::Added(e, _) => Self::Added(e, metadata),
        Self::Removed(e, _) => Self::Removed(e, metadata),
        Self::Moved(e, _) => Self::Moved(e, metadata),
        Self::FieldUpdated(e, _) => Self::FieldUpdated(e, metadata),
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib git::tests`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git mit me && git add src/domain/event.rs src/adapters/event_store/git.rs && git commit -m "Git adapter reads metadata from commit signatures"
```

### Task 9a: Add created_by/created_at to Yak struct

**Files:**
- Modify: `src/domain/yak.rs`
- Modify: `src/domain/field.rs`
- Modify: `src/domain/mod.rs` (re-export METADATA_FIELD)
- Modify: all files that construct `Yak { ... }` (to add default values)

- [ ] **Step 1: Add .metadata.json to RESERVED_FIELDS**

In `src/domain/field.rs`:
```rust
pub const METADATA_FIELD: &str = ".metadata.json";
pub const RESERVED_FIELDS: &[&str] = &[STATE_FIELD, CONTEXT_FIELD, NAME_FIELD, ID_FIELD, METADATA_FIELD];
```

Re-export `METADATA_FIELD` from `src/domain/mod.rs` alongside the other field constants.

- [ ] **Step 2: Add created_by and created_at fields to Yak**

In `src/domain/yak.rs`:
```rust
use super::event_metadata::{Author, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yak {
    pub id: YakId,
    pub name: Name,
    pub parent_id: Option<YakId>,
    pub state: String,
    pub context: Option<String>,
    pub fields: HashMap<String, String>,
    pub children: Vec<YakId>,
    pub created_by: Author,
    pub created_at: Timestamp,
}
```

- [ ] **Step 3: Fix all Yak construction sites with defaults**

Every place that constructs `Yak { ... }` needs the two new fields. Use `Author::unknown()` and `Timestamp::zero()` everywhere for now. Key locations:

1. `src/adapters/yak_store/directory.rs` -- `get_yak()` and `list_yaks()`
2. `src/adapters/yak_store/in_memory.rs` (or wherever InMemoryStorage constructs Yaks)
3. All `#[cfg(test)]` blocks in `src/domain/yak_map.rs` that construct `Yak` in MockStore
4. `tests/features/` -- InProcessWorld if it constructs Yaks

Use `Author::unknown()` and `Timestamp::zero()` as defaults everywhere. We'll wire real values in subsequent tasks.

- [ ] **Step 4: Run full test suite**

Run: `cargo test --lib && cargo test --test cucumber --features test-support`
Expected: All tests pass with default metadata values.

- [ ] **Step 5: Commit**

```bash
git mit me && git add -A && git commit -m "Add created_by/created_at to Yak struct"
```

### Task 9b: Write .metadata.json in disk projection

**Files:**
- Modify: `src/projections/write_to_yak_store.rs`
- Add: `serde_json` to `Cargo.toml` `[dependencies]`

- [ ] **Step 1: Write test for .metadata.json being written**

In `src/adapters/yak_store/directory.rs` tests:
```rust
#[test]
fn test_added_event_writes_metadata_json() {
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

    let (mut storage, _temp) = setup_test_storage();

    let metadata = EventMetadata::new(
        Author { name: "Test".to_string(), email: "test@test.com".to_string() },
        Timestamp(1708300800),
    );
    let event = YakEvent::Added(
        AddedEvent {
            name: Name::from("my yak"),
            id: YakId::from("my-yak-a1b2"),
            parent_id: None,
        },
        metadata,
    );

    storage.on_event(&event).unwrap();

    let dir = storage.base_path.join("my-yak");
    let content = std::fs::read_to_string(dir.join(".metadata.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["created_by"]["name"], "Test");
    assert_eq!(json["created_by"]["email"], "test@test.com");
    assert_eq!(json["created_at"], 1708300800);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib directory::tests::test_added_event_writes_metadata_json`
Expected: FAIL -- .metadata.json not written.

- [ ] **Step 3: Add serde_json dependency**

Add to `Cargo.toml` under `[dependencies]`:
```toml
serde_json = "1"
```

- [ ] **Step 4: Write .metadata.json in projection**

In `src/projections/write_to_yak_store.rs`, update the `Added` handler:
```rust
YakEvent::Added(AddedEvent { name, id, parent_id }, metadata) => {
    self.create_yak(name, id, parent_id.as_ref())?;
    let key = if id.as_str().is_empty() { &YakId::from(name.as_str()) } else { id };
    self.write_field(key, STATE_FIELD, "todo")?;
    self.write_field(key, NAME_FIELD, name.as_str())?;
    // Write .metadata.json with creation author/timestamp
    let metadata_json = serde_json::json!({
        "created_by": {
            "name": metadata.author.name,
            "email": metadata.author.email
        },
        "created_at": metadata.timestamp.as_epoch_secs()
    });
    self.write_field(key, ".metadata.json", &metadata_json.to_string())?;
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --lib`
Expected: All pass, including the new test.

- [ ] **Step 6: Commit**

```bash
git mit me && git add Cargo.toml Cargo.lock src/projections/write_to_yak_store.rs src/adapters/yak_store/directory.rs && git commit -m "Write .metadata.json on Added event in projection"
```

### Task 9c: Read .metadata.json in DirectoryStorage

**Files:**
- Modify: `src/adapters/yak_store/directory.rs`

- [ ] **Step 1: Write test for reading .metadata.json**

In `src/adapters/yak_store/directory.rs` tests:
```rust
#[test]
fn test_get_yak_populates_created_by_and_created_at() {
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

    let (mut storage, _temp) = setup_test_storage();

    let metadata = EventMetadata::new(
        Author { name: "Creator".to_string(), email: "creator@test.com".to_string() },
        Timestamp(1708300800),
    );
    storage.on_event(&YakEvent::Added(
        AddedEvent {
            name: Name::from("my yak"),
            id: YakId::from("my-yak-a1b2"),
            parent_id: None,
        },
        metadata,
    )).unwrap();

    let yak = ReadYakStore::get_yak(&storage, &YakId::from("my-yak-a1b2")).unwrap();
    assert_eq!(yak.created_by.name, "Creator");
    assert_eq!(yak.created_by.email, "creator@test.com");
    assert_eq!(yak.created_at, Timestamp(1708300800));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib directory::tests::test_get_yak_populates_created_by`
Expected: FAIL -- returns `Author::unknown()` defaults.

- [ ] **Step 3: Implement read_metadata helper**

In `src/adapters/yak_store/directory.rs`, add:
```rust
fn read_metadata(&self, dir: &Path) -> (Author, Timestamp) {
    use crate::domain::event_metadata::{Author, Timestamp};

    let metadata_path = dir.join(".metadata.json");
    if let Ok(content) = fs::read_to_string(&metadata_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let author = Author {
                name: json["created_by"]["name"].as_str().unwrap_or("unknown").to_string(),
                email: json["created_by"]["email"].as_str().unwrap_or("").to_string(),
            };
            let timestamp = Timestamp(json["created_at"].as_i64().unwrap_or(0));
            return (author, timestamp);
        }
    }
    (Author::unknown(), Timestamp::zero())
}
```

Use this in both `get_yak()` and `list_yaks()` to populate `created_by` and `created_at` on the returned `Yak`.

- [ ] **Step 4: Run tests**

Run: `cargo test --lib directory`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git mit me && git add src/adapters/yak_store/directory.rs && git commit -m "DirectoryStorage reads .metadata.json for created_by/created_at"
```

### Task 10: snapshot_events reads .metadata.json from git tree

**Files:**
- Modify: `src/adapters/event_store/git.rs`

- [ ] **Step 1: Write test for snapshot_events reading metadata**

```rust
#[test]
fn snapshot_events_reads_metadata_from_tree() {
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

    let (_tmp, mut store) = setup_test_repo();

    let metadata = EventMetadata::new(
        Author { name: "Snapshot Author".to_string(), email: "snap@test.com".to_string() },
        Timestamp(1708300800),
    );

    store.append(&YakEvent::Added(
        AddedEvent {
            name: Name::from("test"),
            id: YakId::from("test-a1b2"),
            parent_id: None,
        },
        metadata.clone(),
    )).unwrap();

    // The Added event handler writes .metadata.json to the tree
    // (via the disk projection - but for git tree, we need to ensure
    // the git adapter also writes it)
    // Actually: the git adapter's build_tree_from_event for Added
    // doesn't currently write .metadata.json. We need to add that.

    let events = store.snapshot_events().unwrap();
    let added = events.iter().find(|e| matches!(e, YakEvent::Added(..))).unwrap();
    assert_eq!(added.metadata().author.name, "Snapshot Author");
    assert_eq!(added.metadata().timestamp, Timestamp(1708300800));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib git::tests::snapshot_events_reads_metadata`
Expected: FAIL

- [ ] **Step 3: Update build_tree_from_event to write .metadata.json for Added**

In the `Added` arm of `build_tree_from_event`:
```rust
YakEvent::Added(e, metadata) => {
    let yak_tree_oid = self.create_yak_tree(e.name.as_str(), "todo", "")?;
    // Add .metadata.json to the yak subtree
    let metadata_json = serde_json::json!({
        "created_by": {
            "name": metadata.author.name,
            "email": metadata.author.email
        },
        "created_at": metadata.timestamp.as_epoch_secs()
    });
    let metadata_blob = self.repo.blob(metadata_json.to_string().as_bytes())?;
    let subtree = self.repo.find_tree(yak_tree_oid)?;
    let mut builder = self.repo.treebuilder(Some(&subtree))?;
    builder.insert(".metadata.json", metadata_blob, 0o100644)?;
    let updated_tree_oid = builder.write()?;

    let path = match &e.parent_id {
        Some(parent) => format!("{}/{}", parent, e.id),
        None => e.id.to_string(),
    };
    self.set_yak_in_root(current_tree, &path, Some(updated_tree_oid))
}
```

- [ ] **Step 4: Update collect_snapshot_events to read .metadata.json**

In `collect_snapshot_events`, after building the Added event, read `.metadata.json` from the subtree:
```rust
let metadata = if let Some(meta_entry) = subtree.get_name(".metadata.json") {
    let meta_blob = self.repo.find_blob(meta_entry.id())?;
    let content = std::str::from_utf8(meta_blob.content())?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        EventMetadata::new(
            Author {
                name: json["created_by"]["name"].as_str().unwrap_or("unknown").to_string(),
                email: json["created_by"]["email"].as_str().unwrap_or("").to_string(),
            },
            Timestamp(json["created_at"].as_i64().unwrap_or(0)),
        )
    } else {
        EventMetadata::default_legacy()
    }
} else {
    EventMetadata::default_legacy()
};
```

Use this metadata for the Added event. For subsequent FieldUpdated events from the same yak in snapshot, use the same metadata (or `default_legacy()` since field updates don't need creation metadata specifically).

- [ ] **Step 5: Run tests**

Run: `cargo test --lib git::tests`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git mit me && git add src/adapters/event_store/git.rs && git commit -m "snapshot_events reads .metadata.json from git tree"
```

## Chunk 4: Hard Reset and Log Display

### Task 11: Update hard reset to pass author/timestamp to AddYak

**Files:**
- Modify: `src/main.rs`
- Modify: `features/reset.feature`
- Modify: `tests/features/steps.rs`

- [ ] **Step 1: Write Cucumber scenario for author preservation through hard reset**

Add to `features/reset.feature` under the "Hard reset" rule:
```gherkin
    Example: Hard reset preserves author in event log
      Given I add the yak "my yak"
      When I hard reset the yaks from disk to git
      And I run yx log
      Then the output should include "<"
```

This verifies that after hard reset, `yx log` output contains the email angle-bracket format (e.g. `<user@example.com>`), proving author metadata was preserved through the round-trip. The `<` character would not appear in the old log format (which only showed event messages).

Note: This test depends on Task 12 (yx log showing author), so it should be run after Task 12 is complete. Write the scenario now but expect it to fail until both tasks are done.

- [ ] **Step 2: Update hard reset code in main.rs**

In the hard reset path, when replaying each yak through AddYak, pass `created_by` and `created_at` from the Yak struct (populated from `.metadata.json` by Task 9c):

```rust
let mut use_case = AddYak::new(yak.name.as_str())
    .with_id(Some(yak.id.as_str()))
    .with_context(yak.context.as_deref())
    .with_author(Some(yak.created_by.clone()))
    .with_timestamp(Some(yak.created_at));
```

- [ ] **Step 3: Run existing hard reset Cucumber tests**

Run: `cargo test --test cucumber --features test-support -- "Hard reset"`
Expected: All existing hard reset scenarios still pass.

- [ ] **Step 4: Commit**

```bash
git mit me && git add src/main.rs features/reset.feature && git commit -m "Hard reset passes author/timestamp to AddYak"
```

### Task 12: Update yx log to show author and timestamp

The spec says "Formatting goes through the display adapter." The current ShowLog uses `println!` directly (bypassing DisplayPort). We'll fix this by routing log output through the display adapter.

**Files:**
- Modify: `src/domain/ports/user_display.rs` (add log_entry method)
- Modify: `src/adapters/user_display/` (implement in ConsoleDisplay and InMemoryDisplay)
- Modify: `src/application/show_log.rs`
- Add: `chrono` to `Cargo.toml` `[dependencies]`
- Modify: `features/log.feature`

- [ ] **Step 1: Write Cucumber scenario for log with author**

Add to `features/log.feature`:
```gherkin
  Rule: Log displays author and timestamp

    @fullstack
    Example: Log entries show author and timestamp
      Given I have a clean git repository
      And I add the yak "test yak"
      When I run yx log
      Then it should succeed
      And the output should include "<"
      And the output should include "Added"
```

The `<` check verifies the new author email format (`name <email>`) is present.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cucumber --features test-support -- "Log entries show author"`
Expected: FAIL - log doesn't show author info yet.

- [ ] **Step 3: Add chrono dependency**

Add to `Cargo.toml` under `[dependencies]`:
```toml
chrono = "0.4"
```

- [ ] **Step 4: Add log_entry method to DisplayPort**

In `src/domain/ports/user_display.rs`, add:
```rust
fn log_entry(&self, author_name: &str, author_email: &str, timestamp: &str, message: &str);
```

Implement in `ConsoleDisplay`:
```rust
fn log_entry(&self, author_name: &str, author_email: &str, timestamp: &str, message: &str) {
    println!("{} <{}>  {}", author_name, author_email, timestamp);
    println!("{}", message);
}
```

Implement in `InMemoryDisplay` (append to output buffer for testability).

- [ ] **Step 5: Update ShowLog to use DisplayPort**

In `src/application/show_log.rs`:
```rust
impl UseCase for ShowLog {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let reader = app
            .event_reader
            .ok_or_else(|| anyhow::anyhow!("Event reader not configured"))?;
        let events = reader.get_all_events()?;

        for (i, event) in events.iter().enumerate() {
            if i > 0 {
                app.display.info(""); // blank line between entries
            }
            let meta = event.metadata();
            let datetime = chrono::DateTime::from_timestamp(
                meta.timestamp.as_epoch_secs(), 0
            ).unwrap_or_default();
            let formatted_time = datetime.format("%Y-%m-%d %H:%M").to_string();

            app.display.log_entry(
                &meta.author.name,
                &meta.author.email,
                &formatted_time,
                &event.format_message(),
            );
        }
        Ok(())
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test --test cucumber --features test-support -- "Log"`
Expected: All log scenarios pass (existing + new).

- [ ] **Step 7: Commit**

```bash
git mit me && git add src/application/show_log.rs src/domain/ports/user_display.rs src/adapters/user_display/ features/log.feature Cargo.toml Cargo.lock && git commit -m "yx log shows author and timestamp via DisplayPort"
```

### Task 13: Run full test suite and final verification

- [ ] **Step 1: Run dev check**

Run: `dev check`
Expected: All checks pass (tests + lint + audit).

- [ ] **Step 2: Fix any remaining issues**

If any tests fail or linting issues exist, fix them.

- [ ] **Step 3: Run the hard reset + log scenario**

Now that both Tasks 11 and 12 are complete, the scenario from Task 11 Step 1 should pass:

Run: `cargo test --test cucumber --features test-support -- "Hard reset preserves author"`
Expected: PASS - log output includes `<` (author email format) after hard reset.

- [ ] **Step 4: Final commit if needed**

```bash
git mit me && git add -A && git commit -m "Fix formatting"
```
