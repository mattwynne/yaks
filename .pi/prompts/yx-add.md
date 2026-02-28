---
description: Create a new sub-yak with context under the current yak
---
Create a new sub-yak called "$@" under the current yak.

Follow these steps:

1. Create the yak:
   ```
   yx add "$@" --under "<current yak name>"
   ```

2. Draft context for the yak and show it to me for review. The
   context should include:
   - Goal: what this yak achieves
   - How: approach or key steps
   - Acceptance criteria: how we know it's done

3. Once I approve, set the context:
   ```
   echo "<context>" | yx context "$@"
   ```
