---
description: Find the worst complexity offender, set a threshold just below it, then refactor to pass
---
Run `dev complexity` to see the current metrics baseline, then:

1. **Tighten thresholds** in `bin/dev` — lower the `max_cognitive` and/or `max_cyclomatic` values so that exactly the worst offender(s) fail. Pick values just below the top function's scores.

2. **Verify the failure** — run `dev complexity` and confirm it now fails, listing the offending function(s).

3. **Refactor** the offending function(s) to bring them under the new thresholds. Extract helper functions, flatten nesting, reduce branching — whatever makes the code clearer.

4. **Verify the fix** — run `dev complexity` again to confirm all functions pass.

5. **Run `dev check`** to make sure nothing is broken.

6. **Commit** with a message describing the new thresholds and what was refactored.

Keep thresholds as tight as possible — the goal is a ratchet that prevents complexity from creeping back up.
