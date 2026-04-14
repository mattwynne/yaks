# Research Brief: Disco + Yaks Integration

## Goal
Answer the open architectural questions so we can write
a clear implementation plan. No code — just decisions.

## Key Discovery

Disco's Forest interface is trivially simple:

```typescript
interface Forest {
    load(callback?: (leaf: Leaf) => void): Promise<Leaf>
    save(tree: Leaf): Promise<void>
}
```

That's it. Two methods. The entire persistence layer is
behind this interface. Writing a `GitForest` adapter that
implements this using isomorphic-git is the integration
point.

## The Big Question: Whose Domain Model?

This is the single most important architectural decision.

### Option A: Disco's model, git as dumb storage

Write a `GitForest` that converts between Disco's
`SerializedLeaf` (nested JSON tree) and a git tree on
`refs/notes/yaks`. Disco's domain model stays untouched.
No WASM. No Rust in the browser.

- Pro: Simplest path. Disco works today. Just swap Forest.
- Pro: No WASM complexity, no bindings, no polyfills.
- Con: Loses yaks' event sourcing and CRDT merge.
- Con: Concurrent edits = last writer wins.
- Con: Two incompatible data formats for the same tree.

### Option B: Yaks' model, Disco as UI shell

Compile yaks' Rust domain to WASM. The browser runs the
full yaks domain (YakMap, events, CRDT merge). Disco's
UI renders the projected state. A `GitForest` adapter
translates between yaks events and the Forest interface.

- Pro: Full event sourcing and conflict-free sync.
- Pro: One data format — CLI and browser read same repo.
- Con: WASM bindings for every domain operation.
- Con: Disco's status model (5 statuses, propagation)
  differs from yaks (3 statuses, no auto-propagation).
- Con: Significant complexity.

### Option C: Thin bridge, parallel evolution

Write a `GitForest` adapter (like Option A) but store
data in yaks' git tree format rather than Disco's JSON.
Map between Disco's Leaf model and yaks' tree shape at
the adapter boundary. No WASM, but the git data is
compatible with the yaks CLI.

- Pro: CLI and browser share the same git data.
- Pro: No WASM needed.
- Pro: Disco's domain model and UI untouched.
- Con: No event sourcing in the browser (snapshot only).
- Con: Status mapping needed (5 ↔ 3).
- Con: Conflict resolution is basic (last push wins on
  the notes ref; no CRDT merge without events).

## Questions That Need Answers

### Q1: Is CLI ↔ browser interop required for v1?

If yes, the git tree format must be compatible. Options
B and C achieve this. Option A does not.

If no, Option A is fastest and we can add interop later.

### Q2: Is concurrent multi-user editing required for v1?

If yes, we need conflict resolution. Option B gives CRDT
merge via event sourcing. Options A and C need a separate
strategy (operational transform, or accept last-writer-wins).

If no, last-writer-wins (fetch before edit, push after)
is fine for all options.

### Q3: How do we map statuses?

Disco: new, doing, done, blocked, canceled (with auto-
propagation up for doing/blocked, down for canceled).

Yaks: todo, wip, done (no auto-propagation).

Options:
a) Map doing→wip, new→todo, ignore blocked/canceled
b) Add blocked/canceled to yaks
c) Let Disco keep its own status model in the browser,
   only sync name/hierarchy/done-ness to git
d) Make status mapping configurable per integration

### Q4: Do we need Disco's real-time collaboration?

Disco's Forest.load(callback) supports Firebase push
updates — multiple users see changes instantly. The
git-based approach is pull-based (fetch on demand).

Options:
a) Polling (fetch every N seconds)
b) GitHub webhooks → WebSocket to browser
c) Accept that git sync is manual/periodic for now

### Q5: What does the test strategy look like?

Disco has good test coverage with null adapters
(NullRealtimeDatabase, NullIdGenerator). The pattern
maps cleanly to a `NullGitForest` for testing.

But integration testing needs:
- Playwright tests against real git (like the spike)
- Or a mock git server for faster feedback

Which matters more: fast tests or realistic tests?

## Recommended Next Step

Have a conversation with Jon Fazzaro about Q1-Q4. His
answers determine which option to pursue. Then write
the implementation plan.

My instinct: Option C gives the best balance. CLI and
browser share data, no WASM complexity, Disco's UI works
as-is. Accept last-writer-wins for v1 and add CRDT merge
later if needed.
