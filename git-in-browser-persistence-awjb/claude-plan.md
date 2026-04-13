# Sprint Plan: Git-in-Browser Persistence Spike

## Goal
Answer: can we read/write yaks events from a browser
using isomorphic-git, without a server?

## Key Risk
Yaks stores events as git notes on `refs/notes/yaks`.
Each commit on that ref has a tree of blobs (one per
yak). isomorphic-git may not support notes natively —
we need to know if the low-level primitives are enough.

## Tasks (in order)

### 1. Verify isomorphic-git primitives (1h)
- Can it clone/fetch a single ref (`refs/notes/yaks`)?
- Can it read trees and blobs from that ref?
- Can it create blobs, trees, and commits on that ref?
- Can it push that ref back to a remote?
- Use a CodeSandbox or simple Vite project.

### 2. Read spike: load yak events in browser (2h)
- Clone a real yaks repo (or just fetch the notes ref)
- Walk the tree on `refs/notes/yaks` HEAD
- Parse the blob contents as yak events
- Render the yak tree in console.log

### 3. Write spike: append an event from browser (2h)
- Create a new blob (e.g. a FieldUpdated event)
- Build a new tree including the new blob
- Create a commit on `refs/notes/yaks` pointing to it
- Push to remote (this is where auth gets interesting)

### 4. Auth spike: push from browser (1h)
- Test with GitHub token in localStorage (throwaway)
- Test with CORS proxy if needed
- Document what auth flow a real app would need

### 5. Write up findings (30m)
- What works, what doesn't, what's painful
- Decision: proceed with isomorphic-git, try a
  different approach, or add a thin server

## Definition of Done
We know whether browser-direct-to-git is viable for
yaks, and if not, what the alternative should be.

## Out of Scope
- WASM bindings (separate yak)
- Disco integration (separate yak)
- Real UI — this is console.log only
