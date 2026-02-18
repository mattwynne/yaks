# 6. Separate move and rename operations

Date: 2026-02-17

## Status

accepted

## Context

The identity model (ADR 0005) introduced immutable IDs so that
events can reference yaks stably across renames and moves. This
raised a design question: should moving and renaming a yak be one
operation or two?

In a filesystem, `mv` combines both: you can change a file's name
and location in a single command. The alternative is to treat move
(changing hierarchy) and rename (changing display name) as
distinct operations with distinct events.

Each operation changes a different attribute of a yak:

- **Move** changes WHERE a yak sits in the hierarchy by updating
  its parent relationship.
- **Rename** changes WHAT a yak is called by updating its display
  name and derived slug.

Neither operation changes the yak's immutable ID (per ADR 0005).

Combining both into a single operation would simplify the CLI at
the cost of blurring two separate domain concepts. Event sourcing
favours single-responsibility events: each event should record one
meaningful state change, making the history easy to read and
replay.

## Decision

Move and rename are separate operations with separate events and
separate CLI commands.

- `yx move <name> --under <parent>` / `yx move <name> --to-root`
  records a `MovedEvent` (changes parent, keeps name and ID).
- `yx rename <old-name> <new-name>` records a `RenamedEvent`
  (changes name and slug, keeps parent and ID).

If a user wants to do both, they run two commands. The yak's
immutable ID is unchanged by either operation.

## Consequences

### What becomes easier

**Single-responsibility events.** Each event type records exactly
one semantic change. `MovedEvent` says "this yak moved to a new
parent"; `RenamedEvent` says "this yak was given a new name". The
audit trail is unambiguous and easy to replay.

**Clearer audit trail.** Reading the event history, it is always
obvious whether a change was a structural reorganisation (move) or
a naming correction (rename). Combined events would obscure this
distinction.

**Self-documenting CLI.** The existence of two separate commands
makes the distinction between hierarchy and naming explicit to
users. There is no ambiguity about which attribute a given command
changes.

### What becomes harder

**Two operations instead of one.** Users who want to both move
and rename a yak must run two commands. This is more explicit but
slightly more verbose.

### Relationship to other ADRs

- **ADR 0002 (CQRS/Event Sourcing)**: Single-responsibility events
  are a natural consequence of the event sourcing model.
- **ADR 0005 (Identity Model)**: Both operations preserve the
  immutable ID introduced in that ADR.
