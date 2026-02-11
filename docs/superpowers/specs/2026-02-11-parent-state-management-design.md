# Parent State Management Design

**Date:** 2026-02-11
**Status:** Draft

## Overview

Implement automatic parent state management for hierarchical yaks. When a child yak's state changes, ancestor states update automatically to reflect work in progress. The system enforces referential integrity by auto-creating missing ancestors and validates that parents cannot be marked done while children remain incomplete.

## Goals

- **Automatic state propagation:** When a child transitions from `todo`, all ancestors become `wip`
- **Referential integrity:** All hierarchical yaks have their complete ancestor chain
- **Strict completion rules:** Parents cannot be marked `done` while children are incomplete
- **Multi-level hierarchy:** Support unlimited depth (`a/b/c/d/...`)
- **Domain-centric design:** Business logic lives in the domain aggregate, not use cases

## Architecture

### Core Design Decision: YakMap Aggregate

Instead of individual `Yak` aggregates, we introduce a single `YakMap` aggregate that owns the entire collection of yaks. This allows the aggregate to enforce hierarchy rules without external queries.

**Why YakMap?**
- Hierarchy rules require knowledge of multiple yaks (parents, children, ancestors)
- Individual yak aggregates would need to query other aggregates (violates DDD principles)
- A collection aggregate has full context to enforce cross-yak rules
- Natural transaction boundary - all related state changes happen atomically

**Trade-off:**
- Loads entire yak collection for every operation
- Acceptable for a CLI tool with reasonable yak counts (<1000s)
- Future optimization: lazy loading or caching if needed

### Component Architecture

```
Use Case Layer
  ↓
Application.with_yak_map(|map| ...)
  ↓
YakMap Aggregate (domain logic)
  - Enforces hierarchy rules
  - Emits events for all mutations
  ↓
Application.save_map()
  ↓
EventBus → EventStore + Projections
```

## Domain Model

### YakMap Aggregate

**File:** `src/domain/yak_map.rs`

```rust
pub struct YakMap {
    yaks: HashMap<String, YakState>,
    pending_events: Vec<YakEvent>,
}

struct YakState {
    state: String,      // "todo" | "wip" | "done"
    context: Option<String>,
}

impl YakMap {
    /// Hydrate from current state projection
    pub fn from_store(store: &dyn Store) -> Result<Self>;

    /// Commands (each emits events)
    pub fn add_yak(&mut self, name: String, context: Option<String>) -> Result<()>;
    pub fn update_state(&mut self, name: String, state: String) -> Result<()>;
    pub fn update_context(&mut self, name: &str, content: String) -> Result<()>;
    pub fn remove_yak(&mut self, name: &str) -> Result<()>;
    pub fn move_yak(&mut self, old_name: &str, new_name: String) -> Result<()>;

    pub fn take_events(&mut self) -> Vec<YakEvent>;
}
```

### Hierarchy Helper Functions

**File:** `src/domain/hierarchy.rs`

Pure functions for working with hierarchical names:

```rust
/// Extract parent name from hierarchical yak name
/// "make tea/get milk" → Some("make tea")
/// "make tea" → None
pub fn get_parent(name: &str) -> Option<String>;

/// Get all ancestor names in order (immediate parent to root)
/// "a/b/c" → ["a/b", "a"]
pub fn get_ancestors(name: &str) -> Vec<String>;

/// Check if one yak is a child of another
/// is_child_of("make tea/get milk", "make tea") → true
pub fn is_child_of(name: &str, potential_parent: &str) -> bool;

/// Find all direct children of a yak
pub fn find_children(parent: &str, yak_states: &HashMap<String, YakState>) -> Vec<String>;
```

## Business Rules

### Rule 1: Auto-Create Ancestor Chain

When adding a hierarchical yak, ensure all ancestors exist.

**Example:**
```bash
yx add "make tea/get milk/find store"
```

**Behavior:**
1. Check if `"make tea"` exists → create if missing (state: `todo`, context: `None`)
2. Check if `"make tea/get milk"` exists → create if missing
3. Create `"make tea/get milk/find store"` with user's context

**Events emitted:**
- `Added { name: "make tea" }` (if created)
- `Added { name: "make tea/get milk" }` (if created)
- `Added { name: "make tea/get milk/find store" }`
- `ContextUpdated { name: "make tea/get milk/find store", content: "..." }` (if provided)

**Implementation:**

```rust
impl YakMap {
    fn ensure_ancestors_exist(&mut self, name: &str) -> Result<()> {
        for ancestor in get_ancestors(name) {
            if !self.yaks.contains_key(&ancestor) {
                self.yaks.insert(ancestor.clone(), YakState {
                    state: "todo".to_string(),
                    context: None,
                });
                self.pending_events.push(YakEvent::Added { name: ancestor });
            }
        }
        Ok(())
    }
}
```

### Rule 2: State Propagation (Child → Ancestors)

When a child's state transitions from `todo`, update all ancestors to `wip`.

**Conditions:**
- Only propagate when transitioning **from** `todo` (not on every state change)
- Only update ancestors that are currently in `todo` state
- Propagate to **all** ancestors (multi-level support)

**Example:**
```bash
yx add "make tea"
yx add "make tea/get milk"
yx state "make tea/get milk" wip
```

**Behavior:**
- `"make tea/get milk"` state: `todo` → `wip`
- `"make tea"` state: `todo` → `wip` (propagated)

**Events emitted:**
- `StateUpdated { name: "make tea/get milk", state: "wip" }`
- `StateUpdated { name: "make tea", state: "wip" }`

**Implementation:**

```rust
impl YakMap {
    pub fn update_state(&mut self, name: String, state: String) -> Result<()> {
        validate_state(&state)?;

        let yak = self.yaks.get(&name)
            .ok_or_else(|| anyhow::anyhow!("Yak '{}' not found", name))?;

        // Check if marking done (validate children first)
        if state == "done" {
            self.validate_children_complete(&name)?;
        }

        // Check if transitioning from todo
        let transitioning_from_todo = yak.state == "todo" && state != "todo";

        // Update this yak
        self.yaks.get_mut(&name).unwrap().state = state.clone();
        self.pending_events.push(YakEvent::StateUpdated {
            name: name.clone(),
            state
        });

        // Propagate to ancestors if transitioning from todo
        if transitioning_from_todo {
            for ancestor in get_ancestors(&name) {
                if let Some(parent) = self.yaks.get_mut(&ancestor) {
                    if parent.state == "todo" {
                        parent.state = "wip".to_string();
                        self.pending_events.push(YakEvent::StateUpdated {
                            name: ancestor,
                            state: "wip".to_string(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
```

### Rule 3: Parent Completion Constraint

Cannot mark a parent as `done` while children are incomplete.

**Validation:**
- When attempting to mark a yak as `done`, check for children
- If any child exists with state != `done`, reject the operation
- Return clear error message

**Example:**
```bash
yx add "make tea"
yx add "make tea/get milk"
yx state "make tea" done  # ERROR: children incomplete
```

**Error message:**
```
Error: Cannot mark 'make tea' as done: children are incomplete
```

**Implementation:**

```rust
impl YakMap {
    fn validate_children_complete(&self, parent_name: &str) -> Result<()> {
        let children = find_children(parent_name, &self.yaks);

        if !children.is_empty() {
            let incomplete = children.iter()
                .any(|name| self.yaks.get(name).unwrap().state != "done");

            if incomplete {
                anyhow::bail!(
                    "Cannot mark '{}' as done: children are incomplete",
                    parent_name
                );
            }
        }

        Ok(())
    }
}
```

### Rule 4: Parent Remains WIP Until Explicit Completion

When all children are marked `done`, the parent remains in `wip` state. The parent must be explicitly marked `done` by the user.

**Rationale:** The parent may represent coordination work beyond just its children. Explicit completion gives users control.

## Application Layer Changes

### Updated Application Struct

**File:** `src/application/app.rs`

Add new method for YakMap operations:

```rust
impl Application<'_> {
    /// Execute a command on the YakMap aggregate
    pub fn with_yak_map<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut YakMap) -> Result<()>,
    {
        // 1. Hydrate aggregate from current state
        let mut yak_map = YakMap::from_store(self.store)?;

        // 2. Execute command
        f(&mut yak_map)?;

        // 3. Save (publish all events)
        self.save_map(&mut yak_map)?;

        Ok(())
    }

    fn save_map(&mut self, yak_map: &mut YakMap) -> Result<()> {
        for event in yak_map.take_events() {
            self.event_bus.publish(event)?;
        }
        Ok(())
    }
}
```

**Note:** The existing `with_yak()` method remains for read-only operations or backwards compatibility during migration.

## Use Case Changes

All use cases become thin wrappers around YakMap commands.

### SetState

**File:** `src/application/set_state.rs`

```rust
impl SetState {
    pub fn execute(&self, app: &mut Application) -> Result<()> {
        app.with_yak_map(|map| {
            map.update_state(self.name.clone(), self.state.clone())
        })
    }
}
```

### AddYak

**File:** `src/application/add_yak.rs`

```rust
impl AddYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let context = app.input.request_content(None, Some(&template))?;

        app.with_yak_map(|map| {
            map.add_yak(self.name.clone(), context)
        })
    }
}
```

### Other Use Cases

Similarly updated:
- `RemoveYak` → `map.remove_yak()`
- `MoveYak` → `map.move_yak()`
- `EditContext` / `ShowContext` → `map.update_context()`
- `DoneYak` → `map.update_state(name, "done")`

## Error Handling

### Non-Existent Yak

Operations on non-existent yaks return clear errors:

```rust
if !self.yaks.contains_key(&name) {
    anyhow::bail!("Yak '{}' not found", name);
}
```

### Invalid State Transitions

Validated by existing `validate_state()` function (accepts only `"todo"`, `"wip"`, `"done"`).

### Parent Completion Constraint

Validated before state change:

```rust
if state == "done" {
    self.validate_children_complete(&name)?;  // Fails fast
}
```

### Event Publishing Failures

All operations are atomic - if any event fails to publish, the entire operation rolls back:

```rust
pub fn with_yak_map<F>(&mut self, f: F) -> Result<()> {
    let mut yak_map = YakMap::from_store(self.store)?;  // Can fail
    f(&mut yak_map)?;  // Command can fail
    self.save_map(&mut yak_map)?;  // Publishing can fail
    Ok(())
}
```

## Testing Strategy

### Unit Tests - YakMap Aggregate

Pure domain logic tests with no infrastructure dependencies:

```rust
#[test]
fn test_update_state_propagates_to_ancestors() {
    let mut map = YakMap::new();
    map.add_yak("parent".to_string(), None).unwrap();
    map.add_yak("parent/child".to_string(), None).unwrap();

    map.update_state("parent/child".to_string(), "wip".to_string()).unwrap();

    assert_eq!(map.yaks.get("parent").unwrap().state, "wip");
    assert_eq!(map.yaks.get("parent/child").unwrap().state, "wip");
}

#[test]
fn test_cannot_mark_parent_done_with_incomplete_children() {
    let mut map = YakMap::new();
    map.add_yak("parent".to_string(), None).unwrap();
    map.add_yak("parent/child".to_string(), None).unwrap();

    let result = map.update_state("parent".to_string(), "done".to_string());

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("children are incomplete"));
}

#[test]
fn test_multi_level_hierarchy_propagation() {
    let mut map = YakMap::new();
    map.add_yak("a".to_string(), None).unwrap();
    map.add_yak("a/b".to_string(), None).unwrap();
    map.add_yak("a/b/c".to_string(), None).unwrap();

    map.update_state("a/b/c".to_string(), "wip".to_string()).unwrap();

    assert_eq!(map.yaks.get("a").unwrap().state, "wip");
    assert_eq!(map.yaks.get("a/b").unwrap().state, "wip");
    assert_eq!(map.yaks.get("a/b/c").unwrap().state, "wip");
}

#[test]
fn test_auto_create_missing_ancestors() {
    let mut map = YakMap::new();

    map.add_yak("a/b/c".to_string(), None).unwrap();

    assert!(map.yaks.contains_key("a"));
    assert!(map.yaks.contains_key("a/b"));
    assert!(map.yaks.contains_key("a/b/c"));

    let events = map.take_events();
    assert_eq!(events.len(), 3); // Added(a), Added(a/b), Added(a/b/c)
}

#[test]
fn test_only_propagate_on_todo_transition() {
    let mut map = YakMap::new();
    map.add_yak("parent".to_string(), None).unwrap();
    map.add_yak("parent/child".to_string(), None).unwrap();

    // First transition: todo → wip (should propagate)
    map.update_state("parent/child".to_string(), "wip".to_string()).unwrap();
    map.take_events(); // Clear events

    // Second transition: wip → done (should NOT propagate)
    map.update_state("parent/child".to_string(), "done".to_string()).unwrap();
    let events = map.take_events();

    // Only one event (for the child), no parent update
    assert_eq!(events.len(), 1);
}
```

### Integration Tests - Use Cases

Test the full stack with in-memory adapters (existing pattern in codebase):

```rust
#[test]
fn test_set_state_use_case_with_hierarchy() {
    let event_store = InMemoryEventStore::new();
    let mut event_bus = EventBus::new(Box::new(event_store));

    let storage = InMemoryStorage::new();
    event_bus.register(Box::new(storage.clone()));

    let display = InMemoryDisplay::new();
    let input = InMemoryInput::new();

    let mut app = Application::new(&mut event_bus, &storage, &display, &input);

    // Add parent and child
    app.handle(AddYak::new("parent")).unwrap();
    app.handle(AddYak::new("parent/child")).unwrap();

    // Update child state
    app.handle(SetState::new("parent/child", "wip")).unwrap();

    // Verify parent was updated
    let parent = storage.get_yak("parent").unwrap();
    assert_eq!(parent.state, "wip");
}
```

### Acceptance Tests - Shellspec

The existing shellspec tests define the acceptance criteria:

**File:** `spec/features/state.sh:50-60`

```bash
It 'sets parent to wip when child state changes from todo'
  When run sh -c "
    yx add 'make tea'
    yx add 'make tea/get milk'
    yx state 'make tea/get milk' wip
    yx list --format markdown
  "
  The line 1 should equal "- [wip] make tea"
  The line 2 should equal "  - [wip] get milk"
End

It 'keeps parent as wip when child is done if other children remain in todo'
  When run sh -c "
    yx add 'make tea'
    yx add 'make tea/get milk'
    yx add 'make tea/boil water'
    yx state 'make tea/get milk' done
    yx list --format markdown
  "
  The line 1 should equal "- [wip] make tea"
  The line 2 should equal $'\e[90m  - [done] get milk\e[0m'
  The line 3 should equal "  - [todo] boil water"
End
```

These tests should pass after implementation by removing the `Skip` lines.

## Migration Strategy

### Phase 1: Create Domain Components

1. Create `src/domain/hierarchy.rs` with helper functions
2. Add unit tests for hierarchy helpers
3. Create `src/domain/yak_map.rs` with YakMap aggregate
4. Add comprehensive unit tests for YakMap

**Validation:** All domain tests pass, no changes to existing behavior.

### Phase 2: Update Application Layer

1. Add `with_yak_map()` method to Application
2. Keep existing `with_yak()` for backwards compatibility
3. Add integration tests

**Validation:** Both patterns work side-by-side.

### Phase 3: Migrate Use Cases (One at a Time)

1. Start with `SetState` (most critical for feature)
2. Then `AddYak` (enables auto-ancestor creation)
3. Migrate remaining use cases
4. Each migration: update use case → run tests → commit

**Validation:** Shellspec tests pass after each migration.

### Phase 4: Remove Skip from Acceptance Tests

1. Remove `Skip` lines from `spec/features/state.sh:50,62`
2. Run shellspec to verify tests pass
3. Commit

**Validation:** All acceptance criteria met.

### Phase 5: Cleanup (Optional)

1. Consider deprecating old `with_yak()` method if no longer needed
2. Update documentation
3. Add ADR if architectural change warrants it

## Trade-offs & Future Considerations

### Benefits
✅ All hierarchy logic in domain aggregate (clean DDD)
✅ No external queries during mutations
✅ Pure, easily testable domain model
✅ Natural transaction boundaries
✅ Simpler than saga/policy approaches

### Costs
❌ Loads entire yak collection for every operation
❌ Aggregate boundary is entire system state
❌ Potential contention in multi-user scenarios (not relevant for CLI)

### Future Optimizations

If performance becomes an issue with large yak collections:

1. **Lazy loading:** Only load affected subgraph (target + ancestors + children)
2. **Caching:** Cache YakMap between operations in long-running processes
3. **Snapshotting:** Periodically snapshot YakMap state to avoid full reconstruction
4. **Incremental updates:** Track dirty subgraphs and only reload changed portions

These optimizations are **not needed now** - implement only if measurements show performance issues.

## Success Criteria

- [ ] All shellspec acceptance tests pass (including currently skipped tests)
- [ ] Unit tests cover all hierarchy rules
- [ ] Integration tests verify use case behavior
- [ ] `yx state child wip` updates all ancestors to `wip`
- [ ] `yx state parent done` fails if children incomplete
- [ ] `yx add parent/child` auto-creates `parent` if missing
- [ ] Multi-level hierarchies work correctly (`a/b/c/d`)
- [ ] Error messages are clear and actionable

## Open Questions

None - design is complete and ready for implementation.

## Next Steps

1. Review and approve this design document
2. Create implementation plan (task breakdown)
3. Create git worktree for isolated development
4. Begin Phase 1: Domain components (TDD approach)
5. Incremental commits after each working piece
