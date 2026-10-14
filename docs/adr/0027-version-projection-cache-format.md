# 27. Version Projection Cache Format

Date: 2026-10-20

## Status

accepted (amends ADR 0026)

## Context

ADR 0026 versions each worktree's `.yaks/` projection by the event revision
recorded in `.projection-tip`. Matching revisions prove that the authoritative
event stream has not changed, but they do not prove that the cached projection
was produced using the current on-disk format.

This matters when a release changes projection behavior without changing any
events. For example, older releases made yak directories read-only. A matching
event revision caused newer releases to accept those directories as current,
leaving worktree deletion broken. Adding a one-off migration for each such
change would accumulate special cases in projection startup.

## Decision

Include a projection cache format version in `.projection-tip`. Version 1 uses
`v1:<event-revision>`, where the event revision is a Git commit ID or `empty`.

A missing, malformed, legacy unversioned, or unsupported checkpoint is a cache
miss. The existing full-rebuild path discards the projection, replays the
authoritative event stream, and writes a checkpoint in the current format.

Bump the projection cache format whenever a release cannot safely interpret or
retain a projection created by the previous format. This version is independent
of the event-store schema version: it describes disposable derived state, not
authoritative events.

## Consequences

- Projection-format changes use the normal cache-rebuild path instead of
  bespoke migrations.
- Existing worktrees rebuild their projection once when first used after this
  change, correcting legacy filesystem permissions as part of normal replay.
- Cache hits continue to require only a ref lookup and checkpoint read.
- A worktree never needs its projection migrated in place; Git remains the sole
  source of truth.
- Alternating between binaries that understand different checkpoint formats is
  safe but may rebuild the projection each time.
- Rebuilding can make the first command after an upgrade slower for large event
  streams.
