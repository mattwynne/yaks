# Sync Should Migrate Remote Data Before Merging

## Problem

When `yx sync` fetches remote data from `refs/remotes/origin/yaks`,
it merges it into the local `.yaks/` directory without checking or
migrating the schema version. If the remote has old-format data
(e.g., V3 nested structure, missing `id`/`name` blobs), that data
gets merged in as-is, corrupting the local V4 state with old-style
yak directories.

The migration system only runs at CLI startup on the local
`refs/notes/yaks` ref. The sync adapter bypasses this entirely.

## Decision

Migrate remote data to the current schema version **before** merging
it with local data during sync.

## Design

### `EventStoreLocation` struct

Introduce a struct to bundle the `(repo, ref_name)` pair that is
currently passed separately (or hardcoded) throughout the migration
system:

```rust
pub struct EventStoreLocation<'a> {
    pub repo: &'a Repository,
    pub ref_name: &'a str,
}
```

### Generalize the migration system

Currently, `read_schema_version`, `write_schema_version`, the
`Migration` trait, and `Migrator` all hardcode `"refs/notes/yaks"`.
Change them to accept an `EventStoreLocation`.

**Updated signatures:**

- `Migration::migrate(&self, location: &EventStoreLocation) -> Result<()>`
- `Migrator::ensure_schema(&self, location: &EventStoreLocation) -> Result<()>`
- `Migrator::run(&self, repo_path: &Path, ref_name: &str) -> Result<()>`
- `read_schema_version(location: &EventStoreLocation) -> Result<Option<u32>>`
- `write_schema_version(location: &EventStoreLocation, version: u32) -> Result<()>`

**Important:** Each concrete migration (`V1->V2`, `V2->V3`,
`V3->V4`) must replace **all** hardcoded `"refs/notes/yaks"`
references with `location.ref_name` — both when reading the
current tree (`refname_to_id`) and when writing the migrated
commit (`repo.commit(Some(location.ref_name), ...)`).

### Existing call sites

`main.rs` currently calls `Migrator::for_current_version().run(root)`.
Updated to: `Migrator::for_current_version().run(root, "refs/notes/yaks")`.
Same behavior, just explicit about the target ref.

### Sync adapter change

In `GitRefSync::sync()`, after `fetch_remote()` succeeds and before
any merge logic, run:

```rust
let remote = EventStoreLocation {
    repo: &self.repo,
    ref_name: "refs/remotes/origin/yaks",
};
Migrator::for_current_version().ensure_schema(&remote)?;
```

The merge then always operates on two V4-format trees, so existing
merge logic works unchanged.

**Note on the remote tracking ref:** `refs/remotes/origin/yaks` is
an ephemeral scratch ref — it's fetched at the start of sync and
deleted at the end (line 491 of `git_ref.rs`). Migration commits
written to this ref are temporary and don't pollute anything.

### Error handling

If migration fails on the remote data, sync aborts with an error.
This is the right default — merging data we can't understand would
corrupt local state. The error message from the migrator is already
descriptive (e.g., "Schema version N is newer than this version of
yx supports").

### Brand-new or V1 remote refs

If the remote ref doesn't exist (`read_schema_version` returns
`None`), migration is skipped — correct, nothing to migrate. If
the remote is V1 (no `.schema-version` blob), `read_schema_version`
returns `Some(1)` and all migrations run in sequence — also correct.

### Test changes

Existing test helpers in `migration.rs` (`read_yak_blob`,
`create_v1_event`, etc.) hardcode `"refs/notes/yaks"`. These
should be generalized to accept a ref name parameter, enabling
tests that verify migration works on non-default refs.

### Cleanup of current mess

Once this fix is deployed, the next `yx sync` will migrate remote
data before merging. Old-style yaks that leaked in can be removed
with `yx rm`. No separate force-push mechanism is needed.

## Consequences

- Migration system becomes reusable across any git ref, not just
  the local event store
- Sync becomes safe across schema version boundaries
- Small increase in sync time (migration check on remote ref), but
  migrations are fast and usually no-ops
- The `EventStoreLocation` struct provides a natural place for
  future helper methods
