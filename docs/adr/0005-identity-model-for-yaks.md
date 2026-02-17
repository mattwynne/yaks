# 5. Identity Model for Yaks

Date: 2026-02-17

## Status

accepted

## Context

The system uses event sourcing (ADR 0002) where all mutations
are recorded as domain events. Events reference yaks by
identity. If a yak is renamed or moved (reparented), those
events must still refer to the same yak unambiguously.

The original implementation used the yak's display name as its
identity. This created several problems:

**Rename breaks history.** When a yak was renamed, all
historical events referencing the old name became orphaned.
There was no stable identifier linking the old and new names
together.

**Move breaks identity.** Reparenting a yak changed its path,
which was also used as identity in events. A `MovedEvent` could
not reference the yak it was moving without the reference
itself changing.

**Names are not filesystem-safe.** Users want human-readable
names with spaces, special characters, and mixed case (e.g.
"Make the tea"). These do not map cleanly to directory names or
git ref names without lossy encoding.

**Three distinct needs exist.** Events need a stable identity
that never changes. On-disk storage needs filesystem-safe
directory names. Users need readable display names. No single
string serves all three purposes well.

ADR 0004 (yak names are leaf-only) separated names from
hierarchy but did not address the fundamental identity problem:
names are mutable, but event references must be immutable.

## Decision

Introduce three distinct concepts for yak identity: ID, Slug,
and Name.

### ID (immutable, event identity)

The ID is the stable identity used in all events. It is
composed of the initial slug plus a short deterministic
suffix (4-char hash of the ancestry path), separated by a
hyphen:

```
make-the-tea-a1b2
refactor-parser-c3d4
```

The ID is assigned once at creation and never changes, even
when the yak is renamed or moved. All domain events
(`AddedEvent`, `StateUpdatedEvent`, `MovedEvent`, etc.)
reference yaks by ID.

The suffix is derived from the ancestry path
(`<parent_id>::<slug>`), ensuring uniqueness: the same name
under different parents produces different IDs.

### Slug (mutable, filesystem identity)

The slug is the slugified form of the current display name.
It is derived deterministically from the name: lowercased,
spaces replaced with hyphens, special characters removed.

```
"Make the tea" -> "make-the-tea"
"Fix bug #42"  -> "fix-bug-42"
```

Slugs are used for on-disk directory names and must be unique
among siblings (yaks sharing the same parent). Directories are
named by slug alone (no random suffix). Each slug directory
contains an `id` file holding the yak's full immutable ID
(e.g. `make-the-tea-a1b2`). This is how the system maps from
filesystem location back to identity. When a yak is renamed,
its slug changes to match the new name (the directory is
renamed), but the `id` file contents remain unchanged.

Slugs are NOT used in events. They are a storage-layer concern
only.

### Name (mutable, display identity)

The name is the free-form display name shown to users. It can
contain spaces, mixed case, and special characters. It is what
appears in `yx ls` output and what users type at the CLI.

```
"Make the tea"
"Fix bug #42"
"Refactor the parser (phase 2)"
```

### Summary

| Concept | Example            | Mutability       | Purpose                  |
|---------|--------------------|------------------|--------------------------|
| ID      | make-the-tea-a1b2  | Immutable        | Stable identity, events  |
| Slug    | make-the-tea       | Changes on rename| Directory names on disk  |
| Name    | Make the tea       | Changes on rename| Display name, free-form  |

### CLI resolution

When a user provides a reference at the CLI, resolution tries
in order:

1. Exact ID match
2. Exact path match (slash-separated slugs)
   **Note:** current implementation resolves paths using names,
   not slugs. Slug-based path resolution is planned.
3. Fuzzy match across IDs and names
   **Note:** current implementation fuzzy-matches against names
   only. Fuzzy matching on IDs is planned.

This allows users to use whichever form is most convenient.
IDs are always unambiguous; names are friendlier for
interactive use.

### Events

`RenamedEvent` and `MovedEvent` are separate events:

- `RenamedEvent` changes a yak's name (and consequently its
  slug). The ID remains unchanged.
- `MovedEvent` changes a yak's parent. The ID remains
  unchanged.

Both events reference the yak by its immutable ID.

### Migration

Existing yaks (created before this identity model) receive
IDs during migration. The suffix is derived from a hash of
the yak's full ancestry path
(`<grandparent_id>::<parent_id>::<leaf_slug>`), making ID
generation deterministic and reproducible regardless of when
or where it runs.

### Display conventions

- Pretty output (`yx ls`) shows names by default.
- Plain output (`yx ls --format plain`) shows IDs for
  scripting and automation.
  **Note:** this is planned behavior. The current
  implementation shows names in plain output as well.

## Consequences

### What becomes easier

**Stable event history.** Events reference yaks by immutable
ID. Renaming or moving a yak does not invalidate any
historical events. The full history of a yak can be
reconstructed by filtering events on its ID.

**Bidirectional traversal.** The Yak struct maintains a
`children: Vec<YakId>` field as a read-model concern,
allowing efficient traversal from parent to children without
scanning all events. This is derived from the event stream
(specifically `AddedEvent.parent_id` and `MovedEvent`) and is
not stored in events itself.

**Filesystem safety.** On-disk directories use slugs, which
are always filesystem-safe. Display names can contain any
characters the user wants without worrying about filesystem
compatibility.

**Name flexibility.** Because display names are separate from
identity and storage, validation rules for names can be
relaxed. Slashes, special characters, and Unicode are all
acceptable in names since directories use slugs.

**CLI ergonomics.** Users can refer to yaks by whichever form
is most natural: the friendly display name for interactive use,
the ID for scripting, or the path for hierarchical navigation.

**Different storage layers use different keys.** The git event
store (source of truth) keys its tree entries by immutable ID
(e.g. `parent-a1b2/child-x1y2/state`). This ensures the tree
structure is stable across renames. The directory storage
(read-model projection, rebuilt from events) keys by slug
(e.g. `parent/child/state`), giving users a browsable
filesystem layout. This divergence is intentional: the event
store optimises for immutability, the projection optimises for
human readability.

### What becomes harder

**Three concepts instead of one.** The single-name approach
was simpler to understand. Contributors now need to understand
the distinction between ID, slug, and name, and when to use
each one.

**Slug uniqueness constraints.** Slugs must be unique among
siblings. Two yaks with names that slugify identically (e.g.
"Make the Tea" and "make-the-tea") cannot coexist under the
same parent. The system must detect and reject these
collisions.

**Display vs. identity mismatch.** Users see names but events
use IDs. Debugging event history requires mapping between the
two, which adds a layer of indirection.

**Migration complexity.** Existing yaks need a one-time
migration to assign IDs. The deterministic hashing approach
ensures migration is reproducible, but it still adds a
migration step that must be tested and documented.

### Relationship to other ADRs

- **ADR 0002 (CQRS/Event Sourcing)**: This ADR provides the
  stable identity model that event sourcing requires. Without
  immutable IDs, event references would break on rename.
- **ADR 0004 (Leaf-only names)**: This ADR builds on the
  separation of names from hierarchy. Names are leaf-only
  display strings; hierarchy is expressed through parent_id
  references using immutable IDs. ADR 0004's terminology was
  corrected alongside this ADR: `parent_id` references an
  immutable ID (YakId), not a slug as originally stated.
