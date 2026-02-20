# Sync as EventStore Responsibility — Implementation Plan

> **For Claude:** REQUIRED: Use superpowers:subagent-driven-development
> (if subagents available) or superpowers:executing-plans to implement
> this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move sync from a separate port into the EventStore,
fixing the duplicate-on-sync bug and making `.yaks/` a one-way
projection.

**Architecture:** EventStore gains a `sync()` method that
exchanges events peer-to-peer. Events flow through the EventBus
so projections update. EventStore is idempotent via event_id.
EventBus is restructured so it doesn't own the EventStore
(resolves Rust ownership conflict for sync).

**Tech Stack:** Rust, git2, existing Cucumber + contract test
infrastructure.

**Spec:** `docs/superpowers/specs/2026-02-19-sync-as-eventstore-responsibility-design.md`
**ADR:** `docs/adr/0007-sync-is-an-eventstore-responsibility-not-a-separate-port.md`
**Examples:** `yx field --show "root cause: migration creates duplicates on sync" examples`

---

## Chunk 1: Add event_id and make append idempotent

### Task 1: Add event_id to EventMetadata

**Files:**
- Modify: `src/domain/event_metadata.rs`
- Modify: `src/adapters/event_store/memory.rs`
- Modify: `src/adapters/event_store/git.rs`
- Test: `src/adapters/event_store/contract_tests.rs`

- [ ] **Step 1: Add `event_id` field to EventMetadata**

Add `event_id: Option<String>` to `EventMetadata`. Optional
because existing events won't have one. Default to `None`.

```rust
// src/domain/event_metadata.rs
pub struct EventMetadata {
    pub author: Author,
    pub timestamp: Timestamp,
    pub event_id: Option<String>,
}
```

Update `EventMetadata::new()` and `default_legacy()` to set
`event_id: None`. Update all call sites that construct
EventMetadata.

- [ ] **Step 2: Fix compilation errors from the new field**

Run `cargo test` — fix any struct literal errors where
EventMetadata is constructed without `event_id`.

- [ ] **Step 3: InMemoryEventStore assigns event_id on append**

When `append()` is called and `event_id` is `None`, assign a
UUID. When `event_id` is already set, preserve it.

```rust
// src/adapters/event_store/memory.rs
fn append(&mut self, event: &YakEvent) -> Result<()> {
    let mut event = event.clone();
    if event.metadata().event_id.is_none() {
        event.set_event_id(Some(uuid::Uuid::new_v4().to_string()));
    }
    self.events.lock().unwrap().push(event);
    Ok(())
}
```

Add a `set_event_id()` method on `YakEvent` (delegates to
metadata).

- [ ] **Step 4: GitEventStore uses commit SHA as event_id**

In `get_all_events()`, set `event_id` to the commit SHA when
parsing events from the git log. In `append()`, the event_id
is set after the commit is created (or left as the SHA of the
new commit).

- [ ] **Step 5: Write contract test for event_id assignment**

```rust
fn appended_events_have_event_id() {
    let mut store = create_store();
    let event = make_added_event("test");
    store.append(&event).unwrap();
    let events = store.get_all_events().unwrap();
    assert!(events[0].metadata().event_id.is_some());
}
```

- [ ] **Step 6: Run tests, commit**

Run: `cargo test --test cucumber --features test-support`
Commit: "Add event_id to EventMetadata"

### Task 2: Make EventStore.append() idempotent

**Files:**
- Modify: `src/adapters/event_store/memory.rs`
- Modify: `src/adapters/event_store/git.rs`
- Test: `src/adapters/event_store/contract_tests.rs`

- [ ] **Step 1: Write contract test for idempotent append**

```rust
fn append_is_idempotent_for_known_event_id() {
    let mut store = create_store();
    let event = make_added_event("test");
    store.append(&event).unwrap();
    let events = store.get_all_events().unwrap();
    let event_with_id = events[0].clone();

    // Append same event again
    store.append(&event_with_id).unwrap();
    let events = store.get_all_events().unwrap();
    assert_eq!(events.len(), 1, "duplicate should be skipped");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p yx --lib contract_tests::append_is_idempotent`
Expected: FAIL (currently appends duplicate)

- [ ] **Step 3: Implement idempotent append in InMemoryEventStore**

Check if any existing event has the same event_id. Skip if so.

```rust
fn append(&mut self, event: &YakEvent) -> Result<()> {
    let mut events = self.events.lock().unwrap();
    if let Some(id) = &event.metadata().event_id {
        if events.iter().any(|e| e.metadata().event_id.as_deref() == Some(id)) {
            return Ok(()); // idempotent skip
        }
    }
    // ... assign event_id if None, push
}
```

- [ ] **Step 4: Implement idempotent append in GitEventStore**

Check if a commit with this SHA already exists in the ref's
history. Skip if so. For events without an event_id (new
events), always append.

- [ ] **Step 5: Run tests, commit**

Run: `dev check`
Commit: "Make EventStore.append() idempotent via event_id"

---

## Chunk 2: Restructure EventBus ownership and add sync to trait

### Task 3: Restructure EventBus so it doesn't own EventStore

Currently `EventBus` owns `Box<dyn EventStore>`. For sync,
we need to call `event_store.sync(peer, &mut event_bus)` which
requires separate ownership. Restructure so Application owns
both.

**Files:**
- Modify: `src/infrastructure/event_bus.rs`
- Modify: `src/main.rs`
- Modify: `src/application/mod.rs` (Application struct)
- Modify: test setup code that creates EventBus

- [ ] **Step 1: Split EventBus into EventBus (listeners only)**

Remove `event_store` from EventBus. EventBus becomes just a
list of listeners with a `notify(&mut self, event: &YakEvent)`
method.

```rust
pub struct EventBus {
    listeners: Vec<Box<dyn EventListener>>,
}

impl EventBus {
    pub fn notify(&mut self, event: &YakEvent) -> Result<()> {
        for listener in &mut self.listeners {
            listener.on_event(event)?;
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Update publish flow**

Wherever `event_bus.publish(event)` is called (use cases),
change to: `event_store.append(&event)?; event_bus.notify(&event)?;`

Or create a helper that does both:
```rust
fn publish(store: &mut dyn EventStore, bus: &mut EventBus, event: YakEvent) -> Result<()> {
    store.append(&event)?;
    bus.notify(&event)?;
    Ok(())
}
```

- [ ] **Step 3: Update main.rs wiring**

Application now holds `&mut dyn EventStore` and `&mut EventBus`
separately.

- [ ] **Step 4: Update all tests that create EventBus**

Fix InProcessWorld and any unit tests.

- [ ] **Step 5: Run full test suite, commit**

Run: `dev check`
Commit: "Restructure EventBus: separate from EventStore"

### Task 4: Add sync() to EventStore trait

**Files:**
- Modify: `src/domain/ports/event_store.rs`
- Modify: `src/adapters/event_store/memory.rs`
- Modify: `src/adapters/event_store/git.rs`

- [ ] **Step 1: Add sync method to EventStore trait**

```rust
pub trait EventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()>;
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
    fn reset_from_snapshot(&mut self, yaks: &[Yak]) -> Result<usize>;
    fn sync(
        &mut self,
        peer: &mut dyn EventStore,
        bus: &mut EventBus,
        output: &dyn OutputPort,
    ) -> Result<()>;
}
```

- [ ] **Step 2: Stub implementations**

Add `todo!()` stubs in GitEventStore, InMemoryEventStore,
and NoOpEventStore so the code compiles.

- [ ] **Step 3: Run tests to verify compilation**

Run: `cargo test` — should compile, existing tests pass
(sync is never called yet).

- [ ] **Step 4: Commit**

Commit: "Add sync() to EventStore trait"

### Task 5: Implement sync in InMemoryEventStore

**Files:**
- Modify: `src/adapters/event_store/memory.rs`
- Test: `src/adapters/event_store/contract_tests.rs`

- [ ] **Step 1: Write contract test — sync no-op**

```rust
fn sync_with_identical_stores_is_noop() {
    let (mut a, mut b) = create_store_pair();
    let event = make_added_event("yak");
    a.append(&event).unwrap();
    b.append(&event).unwrap(); // same event_id
    let mut bus = create_test_bus();
    let output = TestOutput::new();
    a.sync(&mut b, &mut bus, &output).unwrap();
    assert_eq!(a.get_all_events().unwrap().len(), 1);
}
```

- [ ] **Step 2: Write contract test — sync pulls events**

```rust
fn sync_pulls_events_from_peer() {
    let (mut a, mut b) = create_store_pair();
    let event = make_added_event("yak");
    b.append(&event).unwrap();
    let mut bus = create_test_bus();
    let output = TestOutput::new();
    a.sync(&mut b, &mut bus, &output).unwrap();
    assert_eq!(a.get_all_events().unwrap().len(), 1);
    // bus should have received the event
}
```

- [ ] **Step 3: Write contract test — sync pushes events**

```rust
fn sync_pushes_events_to_peer() {
    let (mut a, mut b) = create_store_pair();
    let event = make_added_event("yak");
    a.append(&event).unwrap();
    let mut bus = create_test_bus();
    let output = TestOutput::new();
    a.sync(&mut b, &mut bus, &output).unwrap();
    assert_eq!(b.get_all_events().unwrap().len(), 1);
}
```

- [ ] **Step 4: Write contract test — sync merges both sides**

```rust
fn sync_merges_events_from_both_sides() {
    let (mut a, mut b) = create_store_pair();
    a.append(&make_added_event("yak-a")).unwrap();
    b.append(&make_added_event("yak-b")).unwrap();
    let mut bus = create_test_bus();
    let output = TestOutput::new();
    a.sync(&mut b, &mut bus, &output).unwrap();
    assert_eq!(a.get_all_events().unwrap().len(), 2);
    assert_eq!(b.get_all_events().unwrap().len(), 2);
}
```

- [ ] **Step 5: Write contract test — conflict resolution**

Test per-field last-write-wins and event discard scenarios
from the example map.

- [ ] **Step 6: Write contract test — sync logs events**

Verify output port receives log lines for each event.

- [ ] **Step 7: Implement InMemoryEventStore.sync()**

Exchange events between the two stores. For each event
the local store doesn't have (by event_id), append it
and publish through the bus. Push local events to peer.
Log each event through output port.

- [ ] **Step 8: Run all contract tests, commit**

Run: `dev check`
Commit: "Implement sync in InMemoryEventStore with contract tests"

---

## Chunk 3: Git-native sync and wiring

### Task 6: Implement sync in GitEventStore

**Files:**
- Modify: `src/adapters/event_store/git.rs`
- Test: contract tests from Task 5 (already run against git)

The contract tests from Task 5 already run against both
InMemory and Git (via the macro). So implementing sync in
GitEventStore should make those tests pass for git too.

- [ ] **Step 1: Implement GitEventStore.sync()**

For the initial implementation, use the `peer` parameter's
`get_all_events()` and `append()` methods (event-level sync).
This is not git-native yet but satisfies the contract.

- [ ] **Step 2: Run contract tests against GitEventStore**

Run: `cargo test -p yx --lib event_store`
All sync contract tests should pass for git.

- [ ] **Step 3: Optimise to git-native (optional, can defer)**

If peer is a GitEventStore, use git fetch/merge/push on
`refs/notes/yaks` instead of event-level exchange. Add
`refs/notes/yaks-synced` tracking ref.

- [ ] **Step 4: Commit**

Commit: "Implement sync in GitEventStore"

### Task 7: Wire SyncYaks use case to EventStore.sync()

**Files:**
- Modify: `src/application/sync_yaks.rs`
- Modify: `src/main.rs`
- Modify: `src/application/mod.rs`

- [ ] **Step 1: Update SyncYaks to call event_store.sync()**

```rust
impl UseCase for SyncYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        // Build peer from configured remote
        let mut peer = app.create_sync_peer()?;
        app.event_store.sync(
            peer.as_mut(),
            &mut app.event_bus,
            app.output,
        )?;
        Ok(())
    }
}
```

- [ ] **Step 2: Update main.rs — configure sync peer**

Replace `GitRefSync::new()` with peer configuration.
For git repos, the peer is the origin remote's repo path
(or URL). Store on Application.

- [ ] **Step 3: Run Cucumber sync tests**

Run: `cargo test --test cucumber --features test-support -- features/sync.feature`
All existing sync scenarios should pass.

- [ ] **Step 4: Commit**

Commit: "Wire SyncYaks to EventStore.sync()"

### Task 8: Remove SyncPort and GitRefSync

**Files:**
- Delete: `src/domain/ports/sync.rs`
- Delete: `src/adapters/sync/git_ref.rs`
- Delete: `src/adapters/sync/mod.rs`
- Modify: `src/domain/ports/mod.rs`
- Modify: `src/adapters/mod.rs`
- Modify: `src/application/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Remove SyncPort trait and module**
- [ ] **Step 2: Remove GitRefSync adapter and module**
- [ ] **Step 3: Remove `sync` field from Application struct**
- [ ] **Step 4: Clean up imports and module declarations**
- [ ] **Step 5: Run full test suite**

Run: `dev check`

- [ ] **Step 6: Commit**

Commit: "Remove SyncPort and GitRefSync"

---

## Chunk 4: Update Cucumber features

### Task 9: Flesh out sync Cucumber scenarios

Add scenarios from the example map that aren't already
covered by the existing `features/sync.feature`. Key
additions:

- Conflict resolution (same field, last-write-wins)
- Event discard (remove vs modify)
- Orphaned children (remove parent, child move discarded)
- Sync logging output

**Files:**
- Modify: `features/sync.feature`
- Modify: `tests/features/steps.rs`

- [ ] **Step 1: Add conflict resolution scenarios**
- [ ] **Step 2: Add event discard scenarios**
- [ ] **Step 3: Add sync logging scenarios**
- [ ] **Step 4: Run and make pass**

Run: `cargo test --test cucumber --features test-support`

- [ ] **Step 5: Commit**

Commit: "Add sync conflict and discard Cucumber scenarios"

---

## Deferred

- **Git-native sync optimisation**: Use git fetch/merge/push
  instead of event-level exchange. Task 6 Step 3.
- **CRDT-style event replay**: Smarter conflict resolution
  using event semantics. Future yak.
- **`yx commit`**: Capture manual `.yaks/` edits back into
  event store. Future yak.
