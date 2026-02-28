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
| [0012](0012-compact-after-migration-to-hide-pre-migration-history.md) | Compact after migration to hide pre-migration history | accepted |

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
