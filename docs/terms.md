# Ubiquitous Language — Glossary

> This glossary defines the shared vocabulary for the yak
> project. When naming things in code, tests, CLI, or docs,
> use these terms consistently.
>
> Last reviewed: 2026-03-06

## Nouns

### Ancestor

Any yak in the upward parent chain from a given yak.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| (no dedicated type) | `src/domain/yak_map.rs:107` | `get_ancestor_ids()` returns `Vec<YakId>` |

- **Used in**: `features/add.feature`, `features/state.feature`
- **Related to**: Parent, Descendant, Child

### Author

The person who performed an action, recorded as name and
email from git config.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Author` | `src/domain/event_metadata.rs:2` | Struct with `name` and `email` fields |

- **Used in**: `src/adapters/authentication/`, `features/log.feature`
- **Related to**: Event, Timestamp

Clean — one type, one concept.

### Breadcrumb

The root-first chain of ancestor names displayed when
showing a yak's position in the hierarchy.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Vec<String>` | `src/domain/views.rs:6` | Field on `YakDetailView` |

- **Used in**: `src/application/show_yak.rs:51`
- **Related to**: Ancestor, Yak

A display concept only — no domain type. Only exists as
a field on a view model.

### Child

A yak nested directly under a parent.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Vec<YakId>` | `src/domain/yak.rs` | `children` field on `YakView` |
| `YakChildView` | `src/domain/views.rs:30` | Display struct with `name` and `state` for `yx show` output |

- **Used in**: `features/add.feature`, `features/list.feature`,
  `features/done.feature`
- **Related to**: Parent, Descendant, Sibling
- **Subcategory**: "incomplete children" — children not in
  the done state. Appears in `features/done.feature` Rule
  "Cannot mark a parent as done while children are
  incomplete" and enforced by
  `YakMap::validate_children_complete()`. No type in code.

`YakChildView` carries only `name: String` and
`state: String` — just enough to render a line in the
`yx show` children list. A newcomer might wonder why this
isn't a `YakView`, but `yx show` needs a minimal child
representation, not the full read model.

### Context

Free-form notes attached to a yak — requirements, design
rationale, or any descriptive text. Stored as markdown.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Option<String>` | `src/domain/yak.rs` | Field on `YakView` |
| `Option<String>` | `src/domain/yak_map.rs` | Field on `YakEntry` |
| `CONTEXT_FIELD` | `src/domain/field.rs:7` | Constant `".context.md"` — the reserved field name |

- **Used in**: `features/context.feature`,
  `src/application/edit_context.rs`,
  `src/application/write_context.rs`
- **Related to**: Field

Context is stored as a reserved field (`.context.md`) on
disk but surfaced as a first-class `Option<String>` on both
the read model (`YakView`) and the aggregate's internal
state (`YakEntry`). This duplication exists because context
is important enough to be a direct field rather than forcing
callers to look it up in a generic fields map.

### Descendant

Any yak in the downward subtree from a given yak (children,
grandchildren, etc.).

**Representations in code:**

| Type | File | Role |
|------|------|------|
| (no dedicated type) | `src/application/set_state.rs:36` | Collected via breadth-first traversal |

- **Used in**: `features/move.feature:145`,
  `features/rm.feature:47`
- **Related to**: Child, Ancestor, Subtree

### Event

An immutable record of something that happened to a yak.
Events are the source of truth; the current state of all
yaks is derived by replaying events.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `YakEvent` | `src/domain/event.rs:12` | Enum — the event itself. Variants: `Added`, `Removed`, `Moved`, `FieldUpdated`, `Compacted` |
| `AddedEvent` | `src/domain/events/added.rs:8` | Payload for Added variant |
| `RemovedEvent` | `src/domain/events/removed.rs:8` | Payload for Removed variant |
| `MovedEvent` | `src/domain/events/moved.rs:8` | Payload for Moved variant |
| `FieldUpdatedEvent` | `src/domain/events/field_updated.rs:11` | Payload for FieldUpdated variant |
| `EventFormat` | `src/domain/event_format.rs:6` | Trait for serialising/deserialising event payloads |
| `EventMetadata` | `src/domain/event_metadata.rs:39` | Author + timestamp + optional event_id and commit_sha |
| `LogEntryView` | `src/domain/views.rs:67` | Display model for `yx log` — narrative + relative time + ids |
| `NarrativeSpanView` | `src/domain/views.rs:78` | Display model for a single span in a log entry |

- **Used in**: `src/domain/ports/event_store.rs`,
  `features/log.feature`
- **Related to**: Event Store, Event Stream, Narrative

Nine types. `YakEvent` is the domain type; each variant
has its own payload struct implementing `EventFormat`.
`EventMetadata` is carried on every event but is not an
event itself. `LogEntryView` and `NarrativeSpanView` are
display projections for `yx log`. The variant-per-payload
design is justified — each event carries different data.
The display types exist because log rendering needs
pre-formatted relative times and flat bold flags instead
of enum variants.

Note: legacy event names `Renamed`, `StateUpdated`,
`ContextUpdated` are still parsed for backward
compatibility in `YakEvent::parse()` at
`src/domain/event.rs:71`. These were collapsed into
`FieldUpdated` but old event stores may contain them.

### Event Store

The append-only persistence layer for events.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `EventStore` | `src/domain/ports/event_store.rs:8` | Trait — the port |
| `EventStoreReader` | `src/domain/ports/event_store.rs:31` | Read-only subset of the port |
| git adapter | `src/adapters/event_store/git.rs` | Production: backed by `refs/notes/yaks` |
| memory adapter | `src/adapters/event_store/memory.rs` | Tests: in-memory |

- **Used in**: `src/main.rs`, `features/sync.feature`
- **Related to**: Event, Sync, Yak Store

See the "store" homonym issue in Commentary.

### Event Stream

The ordered sequence of all events. Compaction replaces the
stream with a snapshot.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| (no dedicated type) | — | Implicit: `Vec<YakEvent>` from `EventStore::get_all_events()` |

- **Used in**: `features/compact.feature:2`
- **Related to**: Event, Compact, Snapshot

### Field

A named piece of metadata attached to a yak.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `HashMap<String, String>` | `src/domain/yak.rs` | `fields` on `YakView` |
| `RESERVED_FIELDS` | `src/domain/field.rs:15` | Array of all reserved field name constants |
| `validate_field_name()` | `src/domain/field.rs:53` | Rejects reserved names for writes |
| `validate_field_name_format()` | `src/domain/field.rs:31` | Validates characters (allows reserved for reads) |

- **Used in**: `features/field.feature`,
  `src/application/edit_field.rs`
- **Related to**: Context, Tag, State

**Subcategories** (from `features/field.feature` Rule
"Reserved field names are rejected"):

- **Reserved field**: A dot-prefixed field managed by the
  system. Constants: `.state`, `.context.md`, `.name`,
  `.id`, `.created.json`, `.parent_id`, `.tags`. Users
  cannot write to these via `yx field`. No `ReservedField`
  type exists — the concept lives entirely in the
  `RESERVED_FIELDS` constant and `validate_field_name()`.
- **Custom field**: A user-defined key-value field. No
  `CustomField` type — custom fields are simply strings
  that pass validation. The distinction between reserved
  and custom is enforced by a validation function, not by
  the type system.

### Message

A user-facing feedback item with a severity level.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Message` | `src/domain/views.rs:85` | Enum: `Hint`, `Success`, `Info`, `Warn` |

- **Used in**: `src/domain/ports/user_display.rs`,
  application use cases
- **Related to**: Narrative

Distinct from Narrative — a Message is CLI feedback
("Added yak X"), while a Narrative is an event description
in the log.

### Name

The human-readable display name of a yak. Free-form text,
mutable (can be renamed). Distinct from Slug and Yak ID.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Name` | `src/domain/slug.rs:76` | Newtype wrapper around `String` |

- **Used in**: throughout domain and CLI
- **Related to**: Slug, Yak ID

Clean — one type, one concept. The newtype prevents mixing
up names and IDs at the type level.

### Narrative

A human-readable description of an event, using highlighted
spans for emphasis. Displayed in `yx log`.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Narrative` | `src/domain/narrative.rs:9` | Type alias for `Vec<NarrativeSpan>` |
| `NarrativeSpan` | `src/domain/narrative.rs:3` | Enum: `Plain(String)` or `Highlight(String)` |
| `NarrativeSpanView` | `src/domain/views.rs:78` | Display projection: `text: String` + `bold: bool` |

- **Used in**: `src/application/show_log.rs`,
  `features/log.feature`
- **Related to**: Event, Log

`NarrativeSpan` is the domain type (semantic highlighting).
`NarrativeSpanView` is the display projection (flat struct
for rendering). They carry the same information in different
shapes — the domain type uses enum variants, the view uses
a bool flag.

### Parent

A yak that has children nested under it.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Option<YakId>` | `src/domain/yak.rs` | `parent_id` field on `YakView` |
| `Option<YakId>` | `src/domain/yak_map.rs` | `parent_id` field on `YakEntry` |

- **Used in**: `features/add.feature`, `features/list.feature`
- **CLI flags**: `--under`, `--below`, `--in`, `--into`,
  `--to`, `--blocks`
- **Related to**: Child, Root, Ancestor

The concept "parent" is represented only as an ID reference.
See the `--blocks` issue in Commentary about the flag
aliases.

### Root

A top-level yak with no parent (`parent_id` is `None`).

**Representations in code:**

| Type | File | Role |
|------|------|------|
| (no dedicated type) | — | Identified by `parent_id == None` |

- **Used in**: `features/move.feature:43`,
  `features/add.feature`
- **Related to**: Parent

### Schema Version

The version number of the event store format. Newer binaries
migrate older stores automatically; sync refuses when the
remote uses a newer version than the local binary.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| (not in domain layer) | `src/adapters/event_store/` | Migration modules: `migrate_v1_to_v2.rs` through `migrate_v6_to_v7.rs` |

- **Used in**: `features/migration.feature`,
  `features/sync.feature` (Rule: "Sync refuses when the
  remote uses a newer schema version")
- **Related to**: Event Store, Sync

Exists as a concept in feature files and the migration
infrastructure but has no domain type. An infrastructure
concern that surfaces in user-facing error messages.

### Sibling

Yaks that share the same parent (or are both roots).

**Representations in code:**

| Type | File | Role |
|------|------|------|
| (no dedicated type) | `src/domain/yak_map.rs:121` | `check_sibling_slug_uniqueness()` enforces the constraint |

- **Used in**: `features/list.feature:55`,
  `features/identity.feature`
- **Related to**: Parent, Slug
- **Constraint**: Siblings must have unique slugs

### Slug

A filesystem-safe identifier derived from the name:
lowercase, hyphenated, no special characters. Changes when
a yak is renamed. Used for directory names on disk.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Slug` | `src/domain/slug.rs:42` | Newtype wrapper around `String` |
| `slugify()` | `src/domain/slug.rs:130` | Transforms a name into a slug |

- **Used in**: `features/identity.feature:33`
- **Related to**: Name, Yak ID
- **Subcategory**: "slug collision" — when two sibling yaks
  would have the same slug. Appears in
  `features/identity.feature` Rule "Reject add when slug
  collides with sibling". Enforced by
  `check_sibling_slug_uniqueness()`. No dedicated error
  type — just an `anyhow` error string.

Users never see the word "slug" in the CLI. It's an internal
concept — appropriate for code, not for user output.

### Snapshot

A point-in-time capture of a yak's full state, used during
compaction to replace individual events.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `YakSnapshot` | `src/domain/yak_snapshot.rs:11` | Struct with all yak fields |

- **Used in**: `src/domain/event.rs` (Compacted variant
  carries `Vec<YakSnapshot>`)
- **Related to**: Compact, Event

`YakSnapshot` exists because compaction needs to serialise
complete yak state into a single event. It duplicates the
fields of `YakView` and `YakEntry` — but cannot be either
of them because it serves a different lifecycle moment
(freezing state vs. live read model vs. aggregate
internals). Implements `From<&YakView>` for conversion.

### Readiness

Whether a yak is actionable now. Readiness is derived, not stored as a workflow state.

A yak is ready when it is `todo`, has no incomplete children, has no active yak blockers, and has no manual/external blocker. Parents become ready only after their children are complete; a yak blocker stops blocking once that yak is done. `yx list --ready` filters by this derived value, and JSON list/show output exposes readiness details for agents/scripts, including `ready`, `blocked_by`, and structured readiness reasons.

- **Used in**: `features/list.feature`, `features/show.feature`, `features/blockers.feature`
- **Related to**: State, Blocker, Parent/Child

### Blocker

An explicit reason a yak is not ready.

Blockers come in two forms:
- **Manual/external blocker**: `yx blocker add <yak> --reason "waiting for credentials"`.
- **Yak blocker**: `yx blocker add <yak> --by <blocking-yak> --reason "waiting on it"`.

The `--reason` flag is required for manual blockers and optional for yak blockers. Explicit blocker relationships cannot duplicate blocker relationships already implied by hierarchy, and cannot create cycles.

- **Used in**: `features/blockers.feature`, `features/state.feature`
- **Related to**: Readiness, State, Hierarchy

### State

The lifecycle stage of a yak: todo, wip, or done.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `YakState` | `src/domain/yak_state.rs:9` | Enum: `Todo`, `Wip`, `Done` |
| `STATE_FIELD` | `src/domain/field.rs:6` | Constant `".state"` — the reserved field name on disk |

- **Used in**: `features/state.feature`, `src/main.rs`
  (`yx state`)
- **Related to**: Todo, Wip, Done

`YakState` is a proper enum in the domain, but on disk
it's stored as a string in the `.state` reserved field. `blocked` is a legacy stored state only; current code migrates it to `todo` plus a manual blocker.

**State propagation rules** (from `features/state.feature`
and `features/done.feature`):
- A parent cannot be marked done while children are
  incomplete
- Adding a child to a done parent demotes the parent to
  todo
- Starting work on a child propagates wip to todo ancestors
- A child leaving done demotes done ancestors to wip

These rules are enforced by `YakMap` methods:
`validate_children_complete()`,
`propagate_wip_to_ancestors()`,
`demote_done_ancestors_to_todo()`,
`demote_done_ancestors_to_wip()`.

### Subtree

A yak together with all its descendants.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| (no dedicated type) | — | Collected ad-hoc via traversal |

- **Used in**: `features/rm.feature:47`,
  `features/move.feature:71`
- **Related to**: Descendant

### Tag

A short label attached to a yak for categorisation. Stored
one per line in the `.tags` reserved field. Displayed with
an `@` prefix.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `String` | — | No dedicated type |
| `normalize_tag()` | `src/domain/tag.rs:7` | Strips `@` prefix, validates no whitespace |
| `format_tag()` | `src/domain/tag.rs:22` | Adds `@` prefix for display |
| `TAGS_FIELD` | `src/domain/field.rs:12` | Constant `".tags"` |

- **Used in**: `features/tag.feature`,
  `features/filter_by_tag.feature`
- **Related to**: Field

Tags have no dedicated type — they're `String` values
stored in a reserved field. See "Tag has no dedicated type"
in Commentary.

### Timestamp

The moment an event occurred, stored as epoch seconds.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `Timestamp` | `src/domain/event_metadata.rs:17` | Newtype wrapper around `i64` |

- **Used in**: `src/application/show_log.rs` (rendered as
  relative time, e.g. "2 hours ago")
- **Related to**: Author, Event

### Yak

The fundamental unit of work. Named after "yak shaving" —
discovering that task A requires task B, which requires
task C. Yaks form a tree (parent-child hierarchy), each
with a lifecycle state, optional context, tags, and custom
fields.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `YakView` | `src/domain/yak.rs:15` | Read-model DTO. Has all fields: id, name, parent_id, state, context, fields, tags, children, created_by, created_at |
| `YakEntry` | `src/domain/yak_map.rs:11` | Aggregate internal state. Only: name, parent_id, state, context. No children, no fields, no tags, no created_by |
| `YakSnapshot` | `src/domain/yak_snapshot.rs:11` | Compaction payload. Freezes full state for a Compacted event. Similar to YakView but with fields map and created metadata |
| `YakDetailView` | `src/domain/views.rs:5` | Display model for `yx show`. Breadcrumb, formatted dates, children as `YakChildView`, fields split into short/long |
| `YakChildView` | `src/domain/views.rs:30` | Minimal child display for `yx show`. Just name and state as strings |
| `YakTreeView` | `src/domain/views.rs:37` | Display model for `yx ls`. List of `YakTreeNode`s with format metadata |
| `YakTreeNode` | `src/domain/views.rs:46` | Recursive tree node for `yx ls`. Name, id, state, depth, connector chars, children |
| `validate_yak_name()` | `src/domain/yak.rs:38` | Rejects empty names and null bytes |

That's **seven types** for one concept. Why each exists:

- **`YakView`** is the canonical read model. Queries return
  this. It has everything.
- **`YakEntry`** is the aggregate's internal bookkeeping.
  Deliberately minimal — the aggregate only enforces
  business rules, not serves queries. Lacks children
  (derived from other entries' parent_id), fields, tags,
  and creation metadata.
- **`YakSnapshot`** exists for compaction. Carries everything
  needed to reconstruct a yak from a single event. Close to
  `YakView` but is a serialisation boundary, not a query
  result. Implements `From<&YakView>`.
- **`YakDetailView`**, **`YakChildView`**, **`YakTreeView`**,
  **`YakTreeNode`** are display projections for specific CLI
  commands. They transform domain data into rendering-ready
  shapes (formatted dates, tree connectors, field
  classification).

The first three represent the honest cost of CQRS — the
read model, write model, and serialisation format each need
their own shape. The last four are display concerns. Whether
seven types is too many depends on how often newcomers get
confused about which one to use.

**Subcategories** (from feature files, no types in code):
- "root yak" — `parent_id` is `None`
- "nested yak" — has a parent
- "leaf yak" — implied by prune (removes done *leaves*)
- "done yak" — `state == Done`, eligible for pruning

- **Used in**: everywhere
- **Related to**: Yak ID, Name, State, Context, Field, Tag

### Yak ID

An immutable unique identifier assigned at creation, never
changes — even on rename. Format: `<slug>-<4-char-hash>`.
The hash is derived deterministically from ancestry path
using FNV-1a.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `YakId` | `src/domain/slug.rs:6` | Newtype wrapper around `String` |
| `generate_id()` | `src/domain/slug.rs:167` | Deterministic ID generation from name + parent |

- **Used in**: `features/identity.feature`,
  `docs/adr/0005-identity-model-for-yaks.md`
- **Related to**: Slug, Name

### Yak Map

The in-memory aggregate that enforces all business rules.
Validates state transitions, maintains parent-child
relationships, and emits domain events.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `YakMap` | `src/domain/yak_map.rs:18` | The aggregate root. Contains `HashMap<YakId, YakEntry>`, pending events, and metadata |

- **Used in**: `src/application/command_handler.rs`
- **Related to**: Yak, Event

See "YakMap conflates domain and data structure" in
Commentary.

### Yak Store

The current-state read model persistence layer. Reads and
writes yak data to the `.yaks/` directory on disk.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `ReadYakStore` | `src/domain/ports/yak_store.rs:7` | Trait — read side. Methods: `get_yak`, `list_yaks`, `fuzzy_find_yak_id`, `read_field` |
| `WriteYakStore` | `src/domain/ports/yak_store.rs:14` | Trait — write side. Methods: `create_yak`, `delete_yak`, `rename_yak`, `reparent_yak`, `write_field`, `clear_all` |
| directory adapter | `src/adapters/yak_store/directory.rs` | Production: one directory per yak in `.yaks/` |
| memory adapter | `src/adapters/yak_store/memory.rs` | Tests: in-memory |

- **Used in**: `src/main.rs`, application use cases
- **Related to**: Event Store, Field

See "store" homonym in Commentary.

---

## Verbs

### Add

Create a new yak, optionally nested under a parent.

| Expression | Where | Form |
|------------|-------|------|
| `yx add` | CLI | Command |
| `AddYak` | `src/application/add_yak.rs:16` | Use case |
| `add_yak()` | `src/domain/yak_map.rs:170` | Aggregate method |
| `Added` / `AddedEvent` | `src/domain/event.rs`, `src/domain/events/added.rs:8` | Domain event |

- **Used in**: `features/add.feature`
- **Related to**: Yak, Parent

### Compact

Consolidate the event stream into a single snapshot event,
discarding individual history.

| Expression | Where | Form |
|------------|-------|------|
| `yx compact` | CLI | Command |
| `CompactEvents` | `src/application/compact_events.rs` | Use case |
| `compact()` | `src/domain/ports/event_store.rs` | EventStore method |
| `Compacted` | `src/domain/event.rs` | Domain event variant |

- **Used in**: `features/compact.feature`
- **Related to**: Event Stream, Snapshot, Sync

### Done

Mark a yak as complete.

| Expression | Where | Form |
|------------|-------|------|
| `yx done` (alias `finish`) | CLI | Command |
| `DoneYak` | `src/application/done_yak.rs` | Use case (sugar for SetState with "done") |
| `YakState::Done` | `src/domain/yak_state.rs:9` | State variant |
| `FieldUpdated` on `.state` | `src/domain/event.rs` | Domain event (no dedicated event) |

- **Used in**: `features/done.feature`
- **Related to**: State, Start

### Fuzzy Find

Resolve a partial or case-insensitive yak name to a Yak ID.

| Expression | Where | Form |
|------------|-------|------|
| `fuzzy_find_yak_id()` | `src/domain/ports/yak_store.rs:10` | Port method |

- **Used in**: `features/fuzzy_match.feature`, most use
  cases that accept a yak name
- **Related to**: Name, Yak ID

Not a CLI command — implicit in how the CLI resolves names.
Users experience it but never name it.

### Log

Display the event history as a reverse-chronological
narrative.

| Expression | Where | Form |
|------------|-------|------|
| `yx log` | CLI | Command |
| `ShowLog` | `src/application/show_log.rs` | Use case |

- **Used in**: `features/log.feature`
- **Related to**: Event, Narrative

### Move

Change a yak's parent or promote it to root.

| Expression | Where | Form |
|------------|-------|------|
| `yx move` (alias `mv`) | CLI | Command |
| `MoveYak` | `src/application/move_yak.rs` | Use case |
| `move_yak_to()` | `src/domain/yak_map.rs:484` | Aggregate method |
| `Moved` / `MovedEvent` | `src/domain/event.rs`, `src/domain/events/moved.rs:8` | Domain event |

- **Used in**: `features/move.feature`
- **Related to**: Parent, Root

"Moved" only covers reparenting, not rename. See Commentary.

### Prune

Remove all done leaf yaks in bulk.

| Expression | Where | Form |
|------------|-------|------|
| `yx prune` | CLI | Command |
| `PruneYaks` | `src/application/prune_yaks.rs` | Use case |
| `prune()` | `src/domain/yak_map.rs:425` | Aggregate method |
| `Removed` | `src/domain/event.rs` | Reuses the Removed event |

- **Used in**: `features/prune.feature`
- **Related to**: Done, Remove

### Remove

Delete a specific yak.

| Expression | Where | Form |
|------------|-------|------|
| `yx remove` (alias `rm`) | CLI | Command |
| `RemoveYak` | `src/application/remove_yak.rs` | Use case |
| `remove_yak()` | `src/domain/yak_map.rs:402` | Aggregate method |
| `Removed` / `RemovedEvent` | `src/domain/event.rs`, `src/domain/events/removed.rs:8` | Domain event |

- **Used in**: `features/rm.feature`
- **Related to**: Prune

### Rename

Change a yak's display name without moving it.

| Expression | Where | Form |
|------------|-------|------|
| `yx rename` | CLI | Command |
| `RenameYak` | `src/application/rename_yak.rs` | Use case |
| `rename_yak()` | `src/domain/yak_map.rs:452` | Aggregate method |
| `FieldUpdated` on `.name` | `src/domain/event.rs` | Domain event (no dedicated event) |

- **Used in**: `features/rename.feature`
- **Related to**: Name, Slug

Rename emits `FieldUpdated`, not a dedicated `Renamed`
event. See granularity issue in Commentary.

### Reset

Rebuild either the disk read model from git events, or the
git event log from disk state.

| Expression | Where | Form |
|------------|-------|------|
| `yx reset` | CLI | Command (two modes via flags) |
| `yx reset --disk-from-git` | CLI | Safe: rebuild .yaks/ from events (default) |
| `yx reset --git-from-disk` | CLI | Destructive: wipe events, recreate from .yaks/ |
| `ResetDiskFromGit` | `src/application/reset_disk_from_git.rs` | Use case |
| `ResetGitFromDisk` | `src/application/reset_git_from_disk.rs` | Use case |

- **Used in**: `features/reset.feature`
- **Related to**: Event Store, Yak Store

See Commentary — two opposite operations sharing a name.

### Start

Begin working on a yak (set state to wip).

| Expression | Where | Form |
|------------|-------|------|
| `yx start` (alias `wip`) | CLI | Command |
| `StartYak` | `src/application/start_yak.rs` | Use case (sugar for SetState with "wip") |
| `FieldUpdated` on `.state` | `src/domain/event.rs` | Domain event (no dedicated event) |

- **Used in**: `features/state.feature`
- **Related to**: State, Done

### Sync

Exchange events with peers via git — fetch, merge, push.

| Expression | Where | Form |
|------------|-------|------|
| `yx sync` | CLI | Command |
| `SyncYaks` | `src/application/sync_yaks.rs` | Use case |
| `sync()` | `src/domain/ports/event_store.rs` | EventStore method |

- **Used in**: `features/sync.feature`
- **Related to**: Event Store
- **Conflict strategy**: last-write-wins (from
  `features/sync.feature` Rule)

### Tag (verb)

Add or remove labels on a yak.

| Expression | Where | Form |
|------------|-------|------|
| `yx tag add` | CLI | Subcommand |
| `yx tag remove` (alias `rm`) | CLI | Subcommand |
| `yx tag list` | CLI | Subcommand |
| `AddTag` | `src/application/add_tag.rs` | Use case |
| `RemoveTag` | `src/application/remove_tag.rs` | Use case |
| `ListTags` | `src/application/list_tags.rs` | Use case |
| `FieldUpdated` on `.tags` | `src/domain/event.rs` | Domain event (no dedicated event) |

- **Used in**: `features/tag.feature`
- **Related to**: Tag (noun), Field

---

## Commentary

### Strengths

1. **The identity model is crisp.** Name, Slug, and Yak ID
   are three distinct concepts with clear definitions, backed
   by ADR 0005. Code, tests, and docs all respect the
   distinction.

2. **The three-state lifecycle is used everywhere.** `todo`,
   `wip`, `done` appear identically in the domain model,
   CLI, feature files, and documentation. No drift.

3. **Hierarchy terms are precise.** Parent, child, ancestor,
   descendant, sibling, root — all used correctly and
   consistently across layers.

4. **CLI commands mirror domain verbs.** `add`, `done`,
   `move`, `remove`, `rename` — the CLI surface speaks the
   same language as the domain model. Each verb has a
   consistent expression chain: CLI command -> use case ->
   aggregate method -> domain event.

5. **The "yak" metaphor is carried through completely.** No
   lapses into "task", "item", or "ticket". The metaphor is
   the language.

6. **Newtype wrappers prevent confusion.** `YakId`, `Name`,
   `Slug`, `Timestamp` are all newtypes that make it
   impossible to accidentally pass one where another is
   expected.

7. **Feature files are mostly declarative.** The majority of
   Given/When steps use domain language: `I add the yak`,
   `I mark the yak as done`, `I move the yak under`. The
   Then steps for sync scenarios are particularly good:
   `"alice" should have a yak called "X"`,
   `"bob" yak "X" should have state "done"`.

### Issues Found

#### Feature files mix imperative and declarative styles
- **Type**: Imperative Steps in Feature Files
- **Where**: Most feature files, particularly
  `features/add.feature`, `features/list.feature`,
  `features/move.feature`, `features/cli.feature`
- **Problem**: The features use two parallel step
  vocabularies. Declarative steps express domain intent:
  `When I add the yak "X"`, `When I mark the yak "X" as
  done`. Imperative steps spell out CLI mechanics:
  `When I run yx add X`, `Then the output should include
  "..."`. Both styles coexist in the same feature files,
  sometimes in the same scenario.

  Examples of imperative steps that bypass domain language:
  - `When I run yx add "make the tea" --under "buy milk"`
    — could be `When I add the yak "make the tea" under
    "buy milk"`
  - `Then the output should include "make-the-tea"` —
    asserts on rendered output, not domain state
  - `When I run yx list --format markdown` — could be
    `When I list the yaks in "markdown" format`
  - `When I run yx mv "X" --under "Y"` — tests a CLI
    alias, not a domain operation

  Some imperative steps are legitimate — `features/cli.feature`
  and `features/completion.feature` specifically test CLI
  behaviour and *should* use CLI mechanics. But in domain
  features like `add.feature` and `move.feature`, the `I run
  yx ...` steps test the wiring, not the business rules.

  The dual-mode execution (FullStackWorld runs CLI,
  InProcessWorld calls Rust) explains why both exist. But
  the imperative steps don't exercise the ubiquitous
  language and will break if the CLI surface changes.
- **Suggestion**: For scenarios testing business rules, use
  declarative steps consistently. Reserve `When I run yx`
  for scenarios that specifically test CLI behaviour (help,
  errors, flags, aliases). Consider whether the `I run yx`
  steps could be replaced with declarative equivalents that
  work in both worlds.

#### "reserved field" / "custom field" — unnamed subcategories
- **Type**: Missing Term
- **Where**: `features/field.feature` Rule "Reserved field
  names are rejected", `src/domain/field.rs:15`
  (`RESERVED_FIELDS` constant)
- **Problem**: The feature files name "reserved fields" as a
  concept with business rules around them. The code has a
  `RESERVED_FIELDS` constant and `validate_field_name()`
  function that enforces the distinction. But there's no
  `ReservedField` type or even a named enum distinguishing
  reserved from custom. The concept exists in natural
  language and validation logic but is invisible in the type
  system.
- **Suggestion**: At minimum, add a comment or doc-string
  that names the concept. A `FieldKind` enum
  (`Reserved`/`Custom`) would make the distinction
  type-safe, but the validation function may be sufficient
  given how few call sites there are.

#### FieldUpdated is a catch-all event
- **Type**: Inconsistent Granularity
- **Where**: `src/domain/events/field_updated.rs:11`,
  `src/domain/event.rs:12`
- **Problem**: The FieldUpdated event covers state changes,
  context changes, name changes, tag changes, *and* custom
  fields. Meanwhile, Added, Removed, and Moved have their
  own dedicated event types. The event stream doesn't speak
  the domain language — "Matt changed the state to done" is
  recorded as a field update, not a state transition.
  `FieldUpdatedEvent::format_narrative()` compensates with
  special-case logic for known field names (`.state` ->
  "started"/"finished"/"reset to todo", `.context.md` ->
  "updated context", `.name` -> "renamed", `.tags` ->
  "tagged").
- **Suggestion**: Dedicated events like `StateChanged`,
  `ContextUpdated`, `Renamed`, `Tagged` would better
  reflect the domain. The current design is pragmatic but
  masks intent. Note that backward-compatible parsing
  already handles legacy `Renamed`, `StateUpdated`,
  `ContextUpdated` event names — the code once had finer
  granularity and lost it.

#### Seven types for "Yak"
- **Type**: Inconsistent Granularity
- **Where**: See Yak noun entry above
- **Problem**: `YakView`, `YakEntry`, `YakSnapshot`,
  `YakDetailView`, `YakChildView`, `YakTreeView`,
  `YakTreeNode` all represent "a yak". The first three are
  justified by CQRS. The last four are display projections.
  A newcomer sees seven `Yak*` types and must figure out
  which to use.
- **Suggestion**: The separation exists for real reasons.
  Consider whether the view types (`YakDetailView`,
  `YakTreeView`) could live closer to their rendering code
  in `src/adapters/` rather than in `src/domain/views.rs`,
  since they're display concerns.

#### "--blocks" implies dependency, not containment
- **Type**: Awkward Name
- **Where**: `src/main.rs` (CLI flag alias for `--under`),
  `features/move.feature` Rule: "--below, --in, --into, and
  --blocks are synonyms for --under"
- **Problem**: `--under`, `--below`, `--in`, `--into` all
  describe spatial containment. `--blocks` implies a
  dependency relationship ("this blocks that"). A user might
  expect `--blocks` to mean "this yak is a prerequisite for
  the parent", but it means "nest under".
- **Suggestion**: Remove `--blocks` or clarify whether the
  hierarchy models blocking relationships.

#### "YakMap" conflates domain and data structure
- **Type**: Awkward Name
- **Where**: `src/domain/yak_map.rs:18`
- **Problem**: "Map" is a data structure name (HashMap), not
  a domain concept. The type is the aggregate root. A
  newcomer might think it's just a lookup table.
- **Suggestion**: `YakHerd`, `YakStack`, or simply `Yaks`
  would carry more domain flavour. Low priority.

#### "DAG" in README but "tree" everywhere else
- **Type**: Synonym
- **Where**: `README.md:1` ("DAG-based TODO List")
- **Problem**: The README says DAG but the model enforces a
  tree (each yak has at most one parent). Code, features,
  and CLI all say "tree" and "hierarchy". The README is the
  outlier.
- **Suggestion**: Change to "tree-structured" or
  "hierarchical". Reserve "DAG" for multi-parent support.

#### "store" used for multiple concepts
- **Type**: Homonym
- **Where**: `src/domain/ports/event_store.rs`,
  `src/domain/ports/yak_store.rs`
- **Problem**: "Store" means two things: the append-only
  event log (EventStore) and the current-state read model
  (YakStore). Different purposes in CQRS.
- **Suggestion**: Rename `ReadYakStore`/`WriteYakStore` to
  `YakProjection` or `YakDirectory`.

#### "reset" overloaded with two modes
- **Type**: Homonym
- **Where**: `src/main.rs`, `features/reset.feature`
- **Problem**: `yx reset --disk-from-git` (safe, routine)
  and `yx reset --git-from-disk` (destructive, rare) are
  opposite operations sharing a command name.
- **Suggestion**: Separate commands: `yx rebuild` (safe)
  and `yx reset` (destructive).

#### "Moved" event covers reparenting only
- **Type**: Awkward Name (minor)
- **Where**: `src/domain/events/moved.rs:8`
- **Problem**: "Moved" sounds like it could include rename,
  but it only covers reparenting. Rename is `FieldUpdated`
  on `.name`.
- **Suggestion**: `Reparented` would be more precise.
  `Moved` matches the CLI. Low priority.

#### No term for "the .yaks directory"
- **Type**: Missing Term
- **Where**: `src/adapters/yak_store/directory.rs`,
  `src/README.md`, `features/reset.feature`
- **Problem**: The directory-based read model has no
  consistent name: "the projection", "disk storage",
  "the .yaks directory", "the read model".
- **Suggestion**: Pick one name. "Working copy" or
  "yak directory".

#### Tag has no dedicated type
- **Type**: Missing Term
- **Where**: `src/domain/tag.rs`
- **Problem**: Tags are plain `String` values with helper
  functions. Every other identity concept (`Name`, `Slug`,
  `YakId`, `Timestamp`) has a newtype. Tags are the
  exception — raw strings that could be mixed with other
  strings.
- **Suggestion**: A `Tag` newtype would be consistent with
  the project's pattern and would encapsulate normalisation
  and display logic.

#### "last-write-wins" is unnamed in code
- **Type**: Missing Term
- **Where**: `features/sync.feature` Rule "When both users
  change the same field, the latest change wins"
- **Problem**: The conflict resolution strategy is named in
  feature files but has no corresponding name in the code.
  The merge logic implements it, but there's no
  `ConflictStrategy` type or named constant.
- **Suggestion**: Low priority — the strategy is implicit
  and there's only one. Worth naming if alternatives are
  ever considered.
