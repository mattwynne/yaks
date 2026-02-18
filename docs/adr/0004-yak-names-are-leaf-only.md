# 4. Yak names are leaf-only

Date: 2026-02-15

## Status

accepted

## Context

The system previously synthesized path names like
"parent/child" from the leaf name and the parent path.
This coupled names to hierarchy and leaked internal
storage structure into events. The AddedEvent contained
full path strings (e.g. "parent/child"), and
auto-ancestor creation would implicitly create parent
yaks when adding a nested path.

This made it difficult to reason about events
independently of the storage layout, and violated the
principle that events should be simple, self-contained
facts.

## Decision

Yak names are always leaf-only. Hierarchy is expressed
through parent_id references in events. Internal storage
may still use path keys (InMemoryStorage) or nested
directories (DirectoryStorage), but that is an
implementation detail invisible to the domain layer.

Specifically:
- `YakMap.add_yak` takes `(name, parent_id, context)`
  where name is a leaf name and parent_id references
  the parent yak's immutable ID (YakId)
- `AddedEvent.name` contains only the leaf name
- Auto-ancestor creation is removed from `add_yak`
  (parents must already exist)
- The `move_yak` operation also requires parents to
  exist before moving (no implicit creation)

## Consequences

- Events are simpler and hierarchy-agnostic
- Storage adapters are responsible for their own key
  scheme (path keys, nested directories, etc.)
- Auto-ancestor creation removed from add_yak; parents
  must exist before adding children
- DirectoryStorage needed a new hierarchical name
  resolver to look up yaks by full path names
- Move command uses flag-based syntax (--under <parent>,
  --to-root) instead of path syntax; parents must exist
  before moving (this was revisited separately)
