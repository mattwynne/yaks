# 24. Content-addressed check verification

Date: 2026-05-16

## Status

proposed

## Context

`bin/dev check` currently writes an ignored timestamp file,
`.last-checked`, after the full verification suite passes. Git hooks
and `bin/dev merge` can then call `.githooks/verify-checks` to avoid
running the expensive suite again when files appear older than that
marker.

This timestamp marker is not a trustworthy merge gate:

- It is an ignored local file. Any user or process can `touch` it.
- It is not tied to the commit, tree, merge base, toolchain,
  dependencies, or check script version that was actually verified.
- It mostly detects dirty or staged tracked files newer than the
  marker. A clean feature branch with committed changes can still
  look "already verified" after a rebase, even though those exact
  changes have not been checked in their current form.
- Timestamp comparisons are fragile across worktrees, rebases,
  checkouts, filesystem clock granularity, and generated files.
- It ignores untracked files, even though some checks may depend on
  them.

We still want `dev merge` and agent workflows to be fast. The full
verification suite is deliberately thorough and expensive. Re-running
it unnecessarily after every no-op merge attempt wastes time, but
skipping it based on a mutable timestamp undermines the safety of the
merge path.

ADR 0021 kept workspace concerns explicit and testable through ports
rather than hidden caching. The same principle applies here: if we
cache verification, the cache key must describe the state that was
verified.

## Decision

Replace timestamp-based verification with a content-addressed
verification record.

A successful `bin/dev check` writes a machine-readable record (for
example `.last-checked.json`) containing a fingerprint of the exact
workspace state and verification configuration that passed.

At minimum, the record should include:

- `head`: `git rev-parse HEAD`
- `tree`: `git rev-parse HEAD^{tree}`
- `merge_base_main`: `git merge-base main HEAD` when `main` exists
- `checked_at`: informational timestamp, not used as proof
- `check_version`: a version string or hash for the check policy
- `dev_script_blob`: `git hash-object bin/dev` or equivalent
- `verify_script_blob`: `git hash-object .githooks/verify-checks`
- relevant lock/config fingerprints, initially:
  - `Cargo.lock`
  - `Cargo.toml`
  - `.cargo/config.toml` if present
  - `.pre-commit-config.yaml` if present
  - `.shellspec`

`verify-checks` should trust the marker only when all recorded
fingerprints match the current workspace. It must also require:

- the git working tree has no unstaged tracked changes relevant to
  the check,
- the index has no staged changes relevant to the check,
- optionally, no untracked files in paths that the verification suite
  consumes, or an explicit allowlist for benign untracked paths.

`bin/dev merge` may use this record to skip a full check only after
rebasing the branch onto the current `main`, because the fingerprint
then represents the candidate commit that would be fast-forwarded.

Keep a force path. `bin/dev check --force` should ignore the existing
record, rerun the suite, and overwrite the record on success. Merge
checks can still force specific narrower policies when necessary
(e.g. `.pi` extension-only checks), but they should not force the
full suite simply to work around an untrustworthy marker.

The record remains local and ignored. It is a cache, not a project
artifact. Its trust comes from matching current content, not from
being committed or from its modification time.

## Consequences

### Easier

- `dev merge` can safely skip expensive checks when the exact
  candidate commit and check policy have already passed.
- Re-running a failed or interrupted merge attempt becomes fast once
  the rebased branch has been verified.
- The verification decision becomes explainable: mismatch messages
  can say which fingerprint changed (`HEAD`, `Cargo.lock`, check
  policy, dirty tree, etc.).
- Agents can rely on a deterministic contract instead of mutable
  timestamps.

### Harder

- The check cache needs a small schema and comparison logic.
- The set of inputs to fingerprint must be maintained as the check
  suite changes.
- Some inputs are environmental rather than file-based: Rust
  toolchain, shellspec version, cargo-audit version, OS, feature
  flags. We should decide which are part of `check_version` and
  which are intentionally ignored.
- Untracked files require policy. Strictly rejecting all untracked
  files is safe but noisy; allowing all untracked files is convenient
  but can hide generated fixtures or local files that checks read.

### Follow-up implementation notes

- Prefer JSON over ad-hoc text for the marker so tests can inspect it
  and future fields are easy to add.
- Version the marker schema, e.g. `schema_version: 1`.
- Do not use file mtimes for validity.
- Print a concise reason when verification is reused or rejected.
- Add ShellSpec coverage for:
  - reuse when all fingerprints match,
  - rejection when `HEAD` changes,
  - rejection when `Cargo.lock` or `bin/dev` changes,
  - rejection for dirty tracked files,
  - merge reuse after a verified rebased branch.
