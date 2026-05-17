# Architecture Decision Records

ADRs document significant architectural and design decisions.

## Index

| # | Decision | Status |
|---|----------|--------|
| [0001](0001-migrate-from-bash-to-rust-for-core-implementation.md) | Migrate from bash to Rust | accepted |
| [0002](0002-adopt-cqrs-and-event-sourcing.md) | Adopt CQRS and Event Sourcing | accepted |
| [0003](0003-migrate-acceptance-tests-from-shellspec-to-cucumber.md) | Migrate acceptance tests to Cucumber | accepted |
| [0004](0004-yak-names-are-leaf-only.md) | Yak names are leaf-only (hierarchy via parent_id) | accepted |
| [0005](0005-identity-model-for-yaks.md) | Identity model: ID, Slug, and Name | accepted |
| [0006](0006-separate-move-and-rename-operations.md) | Separate move and rename operations | accepted |
| [0007](0007-sync-is-an-eventstore-responsibility-not-a-separate-port.md) | Sync is an EventStore responsibility | accepted |
| [0008](0008-keep-main-rs-thin-only-wiring-and-routing.md) | Keep main.rs thin: only wiring and routing | accepted |
| [0009](0009-compacted-event-design-and-known-gaps.md) | Compacted event design | accepted |
| [0010](0010-state-reconstruction-mechanisms.md) | State reconstruction mechanisms | accepted |
| [0011](0011-schema-versioning-and-sync-compatibility.md) | Schema versioning and sync compatibility | accepted |
| [0012](0012-compact-after-migration-to-hide-pre-migration-history.md) | Compact after migration to hide pre-migration history | superseded by 0022 |
| [0013](0013-every-cli-command-is-a-use-case.md) | Every CLI command is a UseCase | proposed |
| [0014](0014-no-optimistic-concurrency-on-event-appends.md) | No optimistic concurrency on event appends | accepted |
| [0015](0015-aggregate-loads-from-projected-snapshot-not-event-replay.md) | Aggregate Loads from Projected Snapshot, Not Event Replay | superseded |
| [0016](0016-reserved-fields-stored-as-hidden-files.md) | Reserved fields stored as hidden (dot-prefixed) files | accepted |
| [0017](0017-ratatui-tui-display-adapter.md) | Ratatui TuiDisplay adapter for rich terminal output | accepted |
| [0018](0018-unify-yak-domain-types-into-single-yak-struct.md) | Unify Yak domain types into single Yak struct | accepted |
| [0019](0019-event-source-the-aggregate.md) | Event-source the aggregate | accepted |
| [0020](0020-uuid-for-migration-compaction-events.md) | UUID for Migration Compaction Events | accepted |
| [0021](0021-local-workspace-port-for-onboarding.md) | LocalWorkspacePort for onboarding | accepted |
| [0022](0022-lazy-migration-replaces-boundary-events.md) | Lazy migration replaces boundary events | accepted |
| [0023](0023-agent-aware-help-via-displayport.md) | Agent-aware help via DisplayPort | proposed |
| [0024](0024-content-addressed-check-verification.md) | Content-addressed check verification | proposed |

## When to Write an ADR

Write an ADR when making decisions that:
- Change the architecture or core design patterns
- Introduce new dependencies or technologies
- Affect multiple components or the public API
- Have long-term maintenance implications
- Future maintainers will ask "why did we do it this way?"

Not for: minor implementation details, bug fixes, refactoring,
configuration changes.

## How to Write an ADR

```bash
adrgen create "Title of the Decision"
# Creates docs/adr/NNNN-title-of-the-decision.md
```

Edit the generated file: fill in **Context**, **Decision**, and
**Consequences**. Commit the ADR with the related code changes.

ADRs can reference each other:
- `--supersedes <number>`: replaces an older decision
- `--amends <number>`: modifies an earlier decision
