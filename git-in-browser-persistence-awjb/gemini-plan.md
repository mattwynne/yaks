# Sprint Plan: isomorphic-git in Browser Spike

**Goal:** Test whether isomorphic-git can read and write Yaks
events from a browser without a server, integrating with
jonfazzaro/disco via WASM.

**Tasks & Time Estimates (Total: 32h / 4 days):**

1. **Environment Setup (4h):** Initialize a React/Vite project,
   setup isomorphic-git with lightning-fs (IndexedDB), and
   integrate Yaks WASM.

2. **Ref Management (4h):** Test cloning/fetching a single ref
   (refs/notes/yaks) to answer if partial clones are supported.

3. **Note Reading (4h):** Use git.readNote and tree traversal
   to extract yak event data and verify content against the
   WASM domain.

4. **Note Writing (6h):** Implement git.addNote to create new
   blobs/trees and verify the tree structure follows Yaks
   storage conventions.

5. **Remote Sync & Auth (8h):** Test pushing refs/notes/yaks
   back to a remote (e.g., GitHub via HTTPS) and implement
   Personal Access Token (PAT) authentication.

6. **Integration Prototype (6h):** Connect the isomorphic-git
   backend to the disco (React Tree) frontend using WASM for
   domain logic.

**Definition of Done:**
A functional browser-based prototype that can fetch
refs/notes/yaks, read and display events via WASM, create a
new yak, and push the update back to the remote with proper
authentication.
