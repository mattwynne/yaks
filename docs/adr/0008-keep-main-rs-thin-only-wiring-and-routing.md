# 8. Keep main.rs thin - only wiring and routing

Date: 2026-02-20

## Status

accepted

## Context

The codebase uses hexagonal architecture (ADR 0001) with ports
and adapters. The CLI entry point (`main.rs`) is where adapters
are instantiated and wired together.

Over time, infrastructure logic had leaked into `main.rs`. The
sync command's fetch/push orchestration (git CLI calls to fetch
from origin, create a temporary peer ref, push back after sync,
clean up) lived in `main.rs` rather than in the `GitEventStore`
adapter. This made `main.rs` aware of git transport details and
forced the `EventStore::sync` trait to accept a `peer` parameter
that only existed because the caller was managing the peer
lifecycle.

This pattern would repeat for any adapter that needs external
coordination (e.g. future HTTP-based sync, cloud backends).

## Decision

`main.rs` should only do three things:

1. **Parse CLI arguments** (clap)
2. **Wire adapters to ports** (construct concrete types, inject
   dependencies)
3. **Route commands to use cases** (match on command, call
   `app.handle()`)

Any logic beyond this belongs in an adapter or use case:

- **Transport details** (fetch, push, HTTP calls) go in the
  adapter that owns the connection
- **Orchestration** (retry, cleanup, multi-step workflows) goes
  in the adapter or a use case
- **Domain logic** goes in use cases or the domain layer

As a concrete example: `GitEventStore::sync` now internally
handles fetching from origin, exchanging events with a temporary
peer ref, pushing back, and cleaning up. The `Commands::Sync`
handler in `main.rs` is just `app.handle(SyncYaks::new())`.

## Consequences

- Adapters are self-contained and testable in isolation (e.g.
  git sync tests create real origin repos and verify the full
  fetch/exchange/push cycle)
- `main.rs` stays readable as a table of contents for the CLI
- New sync backends (HTTP, cloud) only need to implement
  `EventStore::sync` without touching `main.rs`
- Adapter tests may be slower (real git repos, network mocks)
  but they test the real thing
- The `EventStore::sync` trait method no longer takes a `peer`
  parameter - each implementation manages its own sync mechanism
