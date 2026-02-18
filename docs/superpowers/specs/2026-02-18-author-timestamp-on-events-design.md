# Author and Timestamp on Events

## Problem

When we round-trip a yak map using `yx reset --git-from-disk
--hard`, the git event history is rebuilt through AddYak. New
git commits are created with the current user and current time,
losing the original author and timestamp information.

Additionally, `yx log` currently shows only event messages with
no indication of who did what or when.

## Design

### Domain Model Changes

New types in `src/domain/`:

```rust
struct Author {
    name: String,
    email: String,
}

struct Timestamp(i64); // Unix epoch seconds

struct EventMetadata {
    author: Author,
    timestamp: Timestamp,
}
```

Every `YakEvent` variant gains a `metadata: EventMetadata`
field. The `YakEvent` enum gets a `fn metadata(&self) ->
&EventMetadata` accessor that delegates to the inner variant.

`YakMap` holds `EventMetadata` at construction time and stamps
it on every event it creates. This is the structural mechanism
for passing author identity to every domain operation that
emits events.

`Yak` domain model gains two non-optional fields:
- `created_by: Author`
- `created_at: Timestamp`

These are set from the `Added` event's metadata during
projection. Non-optional because every yak is created from
an `Added` event which always carries metadata. Use defaults
(`Author { name: "unknown", email: "" }`, `Timestamp(0)`)
for any legacy transition paths.

### New Port: AuthenticationPort

```rust
// src/domain/ports/authentication.rs
pub trait AuthenticationPort {
    fn current_author(&self) -> Author;
}
```

Adapter: `GitAuthentication` reads `user.name` and
`user.email` from git config.

### Application Layer

`Application` constructor takes `&dyn AuthenticationPort`
as a new dependency.

`Application::with_yak_map_result()` constructs
`EventMetadata` (author from port, timestamp from now) and
passes it to `YakMap::new()`.

`AddYak` use case gains two builder methods:
- `.with_author(Author)` — override for replay
- `.with_timestamp(Timestamp)` — override for replay

When set, the Application uses these instead of the defaults
from the port. Other use cases (SetState, EditContext, etc.)
always use current user and time — only AddYak needs overrides
because it's the only use case used during --hard replay.

### Git Adapter Changes

**Writing (`append`):** Use the event's `EventMetadata` to
construct a `git2::Signature::new(name, email, &time)` for
the commit. This means git commits carry the correct author
and time whether it's a fresh operation or a replay.

**Reading (`get_all_events`):** Extract `commit.author().name()`,
`commit.author().email()`, and `commit.time()` from each
git commit. Populate `EventMetadata` on the parsed event.

**`snapshot_events()`:** Read metadata from `.metadata.json`
blob in the yak's git tree (see disk storage below).

### Disk Storage

Each yak directory gains a `.metadata.json` file:

```
.yaks/my-yak/
  context.md
  state
  name
  id
  .metadata.json
```

Contents:

```json
{
  "created_by": {
    "name": "Matt Wynne",
    "email": "matt@example.com"
  },
  "created_at": 1708300800
}
```

Written by the disk projection when handling an `Added` event.
Read by `list_yaks()` to populate `Yak.created_by` and
`Yak.created_at`. Also read by `snapshot_events()` from the
git tree blob.

The dot-prefix keeps it distinct from user custom fields.

### --hard Reset Path

Reads `created_by` and `created_at` from each `Yak` (loaded
from disk via `list_yaks()`), passes them to `AddYak` via
`.with_author()` and `.with_timestamp()`. The git adapter
then creates commits with the original author and timestamp
signatures.

### yx log Display

Git-log style output, one entry per event:

```
Matt Wynne <matt@example.com>  2026-02-15 14:30
Added: "my yak" "my-yak-a1b2"

Matt Wynne <matt@example.com>  2026-02-15 14:32
FieldUpdated: "my-yak-a1b2" "state"
```

Author and timestamp on their own line, blank line between
entries. Formatting goes through the display adapter.
