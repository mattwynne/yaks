## Sprint Plan: `isomorphic-git` Browser Persistence Spike

**Goal**
Prove or disprove that a browser-only app can read and write
Yaks event data on `refs/notes/yaks`, enough to support a
later Disco + Rust/WASM integration.

**Ordered Tasks**
1. Define the minimal persistence contract and fixture repo
   — 0.5 day
   Document the exact git shape to exercise: fetch
   `refs/notes/yaks`, inspect the tip commit, read its tree,
   read per-yak blobs, then append one event by writing a new
   tree and commit on the same ref. Create a tiny fixture
   remote with 2-3 yaks and known contents.

2. Build a browser-only `isomorphic-git` harness — 1 day
   Create the smallest possible static demo or Vite app using
   `isomorphic-git` plus browser storage. Keep it focused on
   git operations and visibility; no Disco or WASM yet.

3. Prove the read path against `refs/notes/yaks` — 1 day
   Test whether the browser can clone/fetch only the notes
   ref, or the narrowest workable equivalent. Resolve the ref
   tip, read the commit, enumerate the tree, and load blob
   contents for selected yaks. Record exactly which
   APIs/options work.

4. Prove the write path on an arbitrary ref — 1 day
   Create/update blobs, write a new tree, create a commit
   with parent = current `refs/notes/yaks` tip, and update
   that ref locally. Verify the resulting commit shape with
   normal git tooling.

5. Prove push + auth to a disposable remote — 1 day
   Push the updated `refs/notes/yaks` back to a test remote.
   Exercise the simplest realistic browser auth first, likely
   HTTPS plus token/credential callback. Capture any CORS,
   provider, or UX blockers.

6. Capture findings and make the decision — 0.5 day
   Summarize supported operations, missing capabilities, auth
   constraints, storage/perf concerns, and recommend one path:
   proceed browser-only, proceed with constraints, or add a
   thin server shim.

**Scope Guards**
- No Disco integration in this spike.
- No Rust-to-WASM work in this spike.
- No performance tuning beyond basic feasibility.
- Test one disposable remote/provider, not many.

**Definition of Done**
- We can answer, with evidence:
  - Can `isomorphic-git` fetch/clone the data needed for
    `refs/notes/yaks` in a browser?
  - Can it read the commit/tree/blob structure Yaks uses?
  - Can it create a new commit on `refs/notes/yaks` without
    a server?
  - Can it push that ref back to a remote?
  - Is there at least one workable browser auth path?
- The spike ends with a binary recommendation:
  `browser-only viable`, `viable with constraints`, or
  `not viable`.
