---
description: Start working on a yak — set context for this session
---
Start working on the yak "$@".

Follow these steps:

1. Get the yak's full details:
   ```
   yx show --json "$@"
   ```
   This gives you id, name, state, context, fields, and children
   in one call.

2. If the context is empty or unclear, ask me to clarify before
   proceeding.

3. Mark it as wip: `yx start "$@"`

4. Remember this yak as the current focus for the session. When
   I use /yx-add or /yx-go, default to this as the parent.

5. Assess readiness of incomplete children:
   - For each incomplete child, run `yx show --json "<child name>"`
     to get its context
   - Categorise them:
     - **Ready to dispatch**: has clear context and acceptance
       criteria, no blockers — could go to a subagent now
     - **Needs context**: exists but context is missing or vague
     - **Blocked**: depends on other incomplete yaks
   - Present the summary and ask what I'd like to do next
