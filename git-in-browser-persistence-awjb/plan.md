# Sprint Plan: isomorphic-git Browser Persistence Spike

## Goal
Prove or disprove that a browser-only app can read and write
yaks events on `refs/notes/yaks`, without a server.

## Key Risk
Yaks stores events as commits on a git notes ref, each with
a tree of blobs. isomorphic-git likely has no native notes
API — we need to know if the low-level primitives (refs,
trees, blobs, commits) are sufficient.

## Tasks

### 1. Create a fixture repo (1h)
Create a tiny test repo with 2-3 yaks and known blob
contents on `refs/notes/yaks`, pushed to a disposable
GitHub remote. This gives us verifiable test data for
every subsequent step.

### 2. Scaffold a browser harness (2h)
Minimal Vite app with isomorphic-git + lightning-fs
(IndexedDB). No React, no WASM, no Disco — just a
page with console output. Focus on git operations and
observability.

### 3. Prove the read path (2h)
- Fetch/clone the fixture repo (can we fetch just
  `refs/notes/yaks`, or do we need the full clone?)
- Resolve the ref, read the commit, walk the tree,
  read blob contents
- Compare against known fixture data
- Record which APIs work and which are missing

**Kill switch:** if we can't read the notes ref tree
at all, stop here and write up findings.

### 4. Prove the write path (2h)
- Create a blob, build a new tree, create a commit
  with parent = current notes ref tip
- Update the ref locally
- Verify with normal git tooling that the commit shape
  matches what yaks expects

### 5. Prove push + auth (2h)
- Push `refs/notes/yaks` back to the fixture remote
- Use GitHub PAT via onAuth callback (simplest path)
- Note any CORS issues or provider-specific blockers

### 6. Write up findings (1h)
- What works, what doesn't, what's painful
- Three-way decision: `browser-only viable`,
  `viable with constraints`, or `not viable`
- If not viable, recommend alternative (thin server
  shim, different persistence, etc.)

## Scope Guards
- No WASM — that's a separate yak
- No Disco integration — that's a separate yak
- No real UI — console.log only
- One remote provider (GitHub), not many
- No performance tuning

## Definition of Done
We can answer each of these with evidence:
1. Can isomorphic-git fetch the notes ref from a browser?
2. Can it read the commit/tree/blob structure yaks uses?
3. Can it create a new commit on that ref?
4. Can it push that ref back?
5. Is there a workable browser auth path?

The spike ends with a written recommendation and the
harness code as proof.

## Total: ~10h (1.5 days)
