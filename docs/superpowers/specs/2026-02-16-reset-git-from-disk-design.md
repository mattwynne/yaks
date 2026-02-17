# Design: yx reset --git-from-disk

## Problem

The git tree at `refs/notes/yaks` can accumulate junk: duplicate
entries (old-style and new-style keys for the same yak), orphaned
children that exist both at root and nested inside parents, and
leftover test data. Currently `yx reset` only goes git-to-disk,
so there's no way to fix the git tree from a cleaned-up disk state.

## Solution

Add a `--git-from-disk` flag to `yx reset` that rebuilds the
`refs/notes/yaks` tree from whatever is on disk in `YAK_PATH`.

```
yx reset                 # git->disk (existing, default)
yx reset --disk-from-git # git->disk (explicit, same as default)
yx reset --git-from-disk # disk->git (new)
```

## Data Flow

`--git-from-disk`:

1. Call `ReadYakStore::list_yaks()` to get all yaks from disk.
   `Yak` must include id, name, state, context, custom fields,
   and children.
2. Call `EventStore::reset_from_snapshot(yaks: &[Yak])` which:
   - Builds a complete git tree recursively (each yak subtree
     contains state, context.md, name, id, and custom field blobs;
     child yaks are nested subtrees)
   - Adds `.schema-version` blob for current schema version
   - Commits to `refs/notes/yaks` with message
     `Snapshot: rebuilt from disk`, parented to current HEAD
3. Prints summary to stdout (e.g. "Snapshot: 12 yaks")

No domain logic or use cases involved. The YakStore reads, the
EventStore writes. This is storage-layer plumbing.

## Event Model

The commit is a Snapshot event. In event sourcing terms this is
a compaction/fold: collapsing history into a single point-in-time
state. The git tree IS the snapshot (no separate Snapshot struct
needed). The schema version is already tracked in `.schema-version`.
The commit message provides provenance.

## Prerequisite

The `Yak` struct must be enriched with:
- `fields: HashMap<String, String>` (custom fields like plan, spec)
- `children: Vec<YakId>` (child yak references)

And `ReadYakStore::list_yaks()` must populate these fields.
This is a separate piece of work that must be done first.

## Files to Modify

### Prerequisite (enrich Yak)
- `src/domain/yak.rs` -- add fields and children to Yak
- `src/adapters/yak_store/directory.rs` -- populate fields
  and children when loading yaks
- `src/domain/ports.rs` -- update if trait signatures change

### Feature (reset --git-from-disk)
- `src/main.rs` -- add --disk-from-git / --git-from-disk flags
  to Reset command, wire up the new path
- `src/domain/ports.rs` -- add reset_from_snapshot to EventStore
- `src/adapters/event_store/git.rs` -- implement reset_from_snapshot

## Testing

- Unit test: `reset_from_snapshot` builds correct git tree from
  a list of yaks (with fields and children)
- Acceptance test: scenario that creates yaks on disk, runs
  `yx reset --git-from-disk`, then `yx reset --disk-from-git`,
  and verifies the round-trip produces the same listing
