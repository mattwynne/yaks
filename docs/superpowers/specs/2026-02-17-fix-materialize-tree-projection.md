# Fix materialize_tree projection

## Problem

`yx reset --disk-from-git` produces directories named by ID
(e.g. `config-7bvf/`) instead of by slug (e.g. `config/`).
This violates ADR 0005 which says on-disk directories should
use slugs, with an `id` file inside holding the immutable ID.

The root cause is `materialize_tree` on `GitEventStore`: it
copies the git tree verbatim to disk using `write_tree_to_dir`,
bypassing the `DirectoryStorage` logic that knows how to create
slug-based directories.

Additionally, older yaks predate the identity model and have
plain-slug IDs (e.g. `dx` instead of `dx-xxxx`). These need
proper IDs generated.

## Design

### Principle: reduce duplication

The directory-naming logic already exists in `DirectoryStorage`
via the `WriteYakStore` trait and `EventListener` projection.
Rather than duplicating that logic in `materialize_tree`, the
reset command should replay synthetic events through the
existing projection.

### New method: `DirectoryStorage::clear()`

Removes all yak directories from the base path. A directory is
a yak if it contains a `context.md` file. Non-yak files (e.g.
`.schema-version`) are preserved. Creates the base directory if
it doesn't exist.

### New method: `GitEventStore::snapshot_events()`

Returns `Result<Vec<YakEvent>>`.

Walks the git tree at HEAD of `refs/notes/yaks` recursively.
For each yak entry (subtree containing a `name` blob):

1. Reads the `name` blob
2. Takes the parent_id from the recursion context (already a
   regenerated ID, or None for root yaks)
3. Calls `generate_id(name, parent_id)` to produce a proper ID
4. Emits `Added { name, id, parent_id }`
5. Emits `StateUpdated { id, state }` if state != "todo"
6. Emits `ContextUpdated { id, content }` if context non-empty
7. Emits `FieldUpdated { id, field_name, content }` for each
   non-reserved file (i.e. not `state`, `context.md`, `name`,
   or `id` — see `RESERVED_FIELDS`)

Parents are processed before children (process blobs before
recursing into child subtrees).

**ID regeneration is intentional.** All IDs are regenerated
from `generate_id(name, parent_id)` regardless of whether the
yak already had a proper ID. This means moved yaks get new IDs
reflecting their current parent. The existing git tree key
(directory name) is ignored as a source of identity. This is a
repair operation — the subsequent `--git-from-disk` step
rebuilds the git tree with the new IDs, creating a clean break
from old event history.

**No `id` blob dependency.** Event-sourced git tree entries
(created via `yx add`) do not contain `id` blobs — only
`name`, `state`, and `context.md`. The `id` blob is only
present for entries written by `reset_from_snapshot`. Since
`snapshot_events` regenerates all IDs from `name` + parent
context, it does not read `id` blobs at all.

### Deleted methods

- `GitEventStore::materialize_tree()`
- `GitEventStore::write_tree_to_dir()`

### Updated: reset command in `main.rs`

The `--disk-from-git` branch becomes:

```rust
let event_store = GitEventStore::new(root)?;
let events = event_store.snapshot_events()?;
storage.clear()?;
for event in &events {
    storage.on_event(event)?;
}
```

### Repair cycle

To fully repair inconsistent data:

```
yx reset --disk-from-git   # rebuild disk with proper IDs + slugs
yx reset --git-from-disk   # rebuild git tree from corrected disk
```

This creates a discontinuity in the commit history: old commits
reference old IDs, while the new snapshot commit uses
regenerated IDs. This is intentional — the snapshot supersedes
the old event history.

## What does NOT change

- `WriteYakStore` / `EventListener` / `write_to_yak_store.rs`
- `reset_from_snapshot` (the `--git-from-disk` path)
- `generate_id` / `slugify`
- The git event store tree format (still keyed by ID)
