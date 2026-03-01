---
description: Dispatch a yak to a subagent in its own worktree
---
Dispatch the yak "$@" to a subagent for implementation.

Follow these steps:

1. Get the yak's full details:
   ```
   yx show --format json "$@"
   ```

2. If the context is empty or insufficient for a subagent to work
   independently, add your questions to the yak:
   ```
   echo "your questions here" | yx field "$@" questions
   ```
   Then STOP and tell me the yak needs more context before dispatch.

3. Mark it as wip: `yx start "$@"`

4. Create a worktree using the yak's ID (from the JSON output):
   ```
   mkdir -p .worktrees
   git worktree add .worktrees/<yak-id> -b <yak-id>
   ```

5. Pre-digest context for the subagent: read any source files
   referenced in the yak context and extract the relevant code
   snippets with file paths and line numbers. The subagent should
   be able to start implementing immediately without exploring
   the codebase.

6. Spawn a background subagent with a task that includes:
   - The worktree path
   - Instruction to run `yx reset` first to populate .yaks in
     the worktree (the event store is shared via git, so yx
     commands then work locally)
   - The full yak context
   - Pre-digested code snippets from step 5
   - Steps: implement, run tests, commit to the worktree branch
     with a message referencing the yak ID
   - Instruction to mark done when finished: `yx done "$@"`
   - Instruction NOT to merge — we merge it with `dev merge <branch>`
