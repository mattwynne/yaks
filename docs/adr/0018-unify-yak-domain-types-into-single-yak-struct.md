# 18. Unify Yak domain types into single Yak struct

Date: 2026-03-07

## Status

Accepted

## Context

The domain layer had three overlapping types representing yaks:

- **`YakView`** (in `yak.rs`) — a read-model DTO with all fields including `children: Vec<YakId>`, used by stores and the presentation layer.
- **`YakEntry`** (in `yak_map.rs`) — a slimmer internal type used by the `YakMap` aggregate, carrying only `name`, `parent_id`, `state`, and `context`.
- **`YakSnapshot`** (in `yak_snapshot.rs`) — used inside `Compacted` events, with most fields but missing `tags`.

This created several problems:
- **Duplication**: the same conceptual entity was represented three times with overlapping but inconsistent field sets.
- **Mapping overhead**: `YakMap::from_store()` had to convert from `YakView` to `YakEntry`, discarding fields like `fields`, `tags`, `created_by`, and `created_at`.
- **Fragile evolution**: adding a new field to a yak meant updating up to three structs and their conversion code.
- **Misleading names**: `YakView` was described as a "DTO" but was used as the canonical domain type returned by stores.

## Decision

Replace all three types with a single `Yak` struct:

```rust
pub struct Yak {
    pub id: YakId,
    pub name: Name,
    pub parent_id: Option<YakId>,
    pub state: YakState,
    pub context: Option<String>,
    pub fields: HashMap<String, String>,
    pub tags: Vec<String>,
    pub created_by: Author,
    pub created_at: Timestamp,
}
```

Key design choices:

1. **`children` is excluded** from the struct. It was the only derived/computed field on `YakView` — populated by scanning sibling yaks for matching `parent_id`. It is now computed at the presentation layer when building view models (`YakDetailView`, `YakTreeNode`).

2. **`id` is stored inside the struct** even though `YakMap` also uses it as the HashMap key. This redundancy is intentional for convenience — code that receives a `Yak` always has access to its id without needing the map context.

3. **`YakSnapshot` becomes a type alias** (`pub use Yak as YakSnapshot`) for backward compatibility with `Compacted` event code. The git tree doesn't store `tags`, so snapshots read from the tree get `tags: vec![]`.

4. **No impact on persisted events.** Events are text-based strings (`"Added: \"name\" \"id\""`). The Rust types are in-memory representations only.

## Consequences

### Easier
- **Adding fields**: a new yak property only needs to be added in one place.
- **Understanding the model**: one type to learn, no mental mapping between YakView/YakEntry/YakSnapshot.
- **Aggregate consistency**: `YakMap` now holds full `Yak` objects, so operations like `from_store()` are a simple clone rather than a lossy projection.

### Harder
- **`children` must be computed**: code that previously accessed `yak.children` now needs to derive it by scanning for matching `parent_id`. This is a small cost paid at the presentation layer rather than baked into the domain type.
- **Larger aggregate entries**: `YakMap` entries now carry `fields`, `tags`, and metadata even though the aggregate's command logic doesn't use them. This is a minor memory trade-off for a simpler model.
