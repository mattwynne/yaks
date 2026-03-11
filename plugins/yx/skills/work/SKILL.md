---
name: work
description: Pick up ready leaf yaks and implement them — dispatch subagents in isolated worktrees, then merge back to main
---

# /yx:work — Working on Yaks

## Overview

**You orchestrate; subagents implement.** Each leaf yak gets its own worktree and branch. When the subagent finishes and verification passes, you merge back to main.

**The golden rule: Neither you nor your subagents may modify main's working tree.**

When invoked with arguments (e.g., `/yx:work fix auth bug`), use those to identify which yak to work on.

**Always announce:** "I'm using `/yx:work` to implement leaf yaks in isolated worktrees. I'll merge back to main once verification passes."

## When to Use

Use when:
- User asks to "implement", "work on", or "pick up" yaks
- Ready leaf yaks exist (has context, no @needs-human tag, no children)
- User wants to start building after mapping or preparing

Don't use when:
- No leaf yaks exist yet (use `/yx:map` first)
- Yaks lack context or clear definition of done (use `/yx:prepare` first)
- User is just exploring or planning (not ready to implement)

## What This Skill Owns vs What the Project Owns

### This Skill Owns
- Yak lifecycle (identify ready leaves, mark wip/done, handle @needs-human tags)
- Worktree management (create, cleanup, ensure main is never modified)
- Orchestration (dispatch subagents, review output, determine independence for parallel work)
- Merging (verify, merge to main, clean up branches)

### The Project Owns
- **Implementation practices:** TDD, code review, commit messages, etc. — defer to project conventions
- **Verification:** Before merging, run the project's verification (tests, linting, CI checks). Check CLAUDE.md for "done" criteria
- **Code quality:** The subagent decides how to implement, what tests to write, how to refactor

**The key:** This skill provides structure; the project provides standards. Always check CLAUDE.md or project docs for verification requirements.

## THE MAIN BRANCH RULE

**Neither you nor your subagents may modify main's working tree.** Main is for merging finished branches only.

- Subagents work in their own worktrees
- You (orchestrating agent) never edit files on main
- If uncommitted changes exist on main, STOP and ask the user

Why: Keeps main clean, prevents conflicts, makes rollback simple, enforces trunk-based development.

## Before Starting Work

**Always verify before starting:**

1. **Context exists:** `yx show "yak name" --format json` — if missing/vague, STOP and tell user to use `/yx:prepare`
2. **No @needs-human tag:** If tagged, surface the question and ask user to answer or pick a different yak
3. **Is a leaf:** No children. If not a leaf: "This yak has incomplete children. Ready leaves: [list them]."

Good context includes: what needs doing, why it matters, what "done" looks like, and any constraints.

## Single Yak Workflow

**Step 1: Mark as WIP**
```bash
yx start "yak name"
```

**Step 2: Create Worktree**
```bash
yx show "yak name" --format json  # Get the slug/ID (e.g., "fix-auth-bug-a1b2")
git worktree add .worktrees/<yak-slug> -b <yak-slug>
```

**Step 3: Dispatch Subagent**
```typescript
const yakContext = await getYakContext("yak name")
const prompt = `${yakContext}

Implement this yak. Commit when done. Follow any conventions in CLAUDE.md or project docs.`

await subagent({
  agent: "claude",
  task: prompt,
  cwd: `.worktrees/<yak-slug>`  // Work in isolated worktree
})
```

The subagent reads context as spec, implements, commits. You don't interfere — just wait.

**Step 4: Review Subagent Output**

When subagent returns: read what it did, check commits, verify it addressed acceptance criteria. If stuck:
```bash
yx tag "yak name" @needs-human
# Add question to context, pick another leaf
```

**Step 5: Run Verification**
```bash
cd .worktrees/<yak-slug>
npm test && npm run lint  # Adjust to project (cargo test, pytest, etc.)
# Check CLAUDE.md for project-specific requirements
```

If verification fails: dispatch subagent again with failure info, or tag @needs-human. Do not merge failing branches.

**Step 6: Merge to Main**
```bash
cd <back to main>
git checkout main
git merge --ff-only <yak-slug>  # Rebase first if main has moved
git push origin main
```

**Step 7: Clean Up**
```bash
git worktree remove .worktrees/<yak-slug>
git branch -d <yak-slug>
```

**Step 8: Mark Done**
```bash
yx done "yak name"
```

## Multiple Independent Yaks (Parallel Work)

Work on multiple leaves in parallel if they're independent (don't touch same files or depend on each other).

**Step 1: Identify Independent Leaves**

Use `yx list` to find ready leaves. Check context to ensure:
- Different files/areas (e.g., "Add profile endpoint" + "Update README")
- No dependencies on each other

Dependent examples (work sequentially): "Add migration" + "Use new schema", "Refactor auth" + "Add auth tests"

When in doubt, work sequentially.

**Step 2: Create Worktrees**
```bash
yx start "yak 1"
git worktree add .worktrees/<slug-1> -b <slug-1>
yx start "yak 2"
git worktree add .worktrees/<slug-2> -b <slug-2>
```

**Step 3: Dispatch in Parallel**
```typescript
await subagent({
  tasks: [
    { agent: "claude", task: yakPrompt1, cwd: `.worktrees/slug-1` },
    { agent: "claude", task: yakPrompt2, cwd: `.worktrees/slug-2` }
  ]
})
```

**Step 4: Verify and Merge Each**

Review output, verify, merge, clean up, mark done (same as single workflow). If conflicts arise: merge first, rebase second, dispatch subagent again if needed.

## @needs-human Convention

**Tag when:** context unclear, subagent stuck, verification fails unexpectedly, design question arises.

**When encountering tagged yak:**
"This yak is tagged @needs-human. Question: [from context]. Answer now or pick a different yak?"

**To add tag:**
```bash
yx tag "yak name" @needs-human
yx context "yak name" <<'EOF'
[Existing context]

## Open Question
[What's blocking progress]
EOF
```

**To remove after answer:**
```bash
yx context "yak name" <<'EOF'
[Updated context with answer]
EOF
yx tag "yak name" --remove @needs-human
```

## Worktree Lifecycle Reference

Complete lifecycle for a single yak:

```bash
# 1. Verify yak is ready
yx list                                    # Find leaf yaks
yx show "yak name" --format json           # Get slug and check context

# 2. Start work
yx start "yak name"                        # Mark as WIP
git worktree add .worktrees/<slug> -b <slug>

# 3. Implement (via subagent in worktree)
# [Subagent works, commits changes]

# 4. Verify
cd .worktrees/<slug>
npm test && npm run lint                   # Or project-specific commands
cd <back to main>

# 5. Merge
git checkout main
git merge --ff-only <slug>                 # May need rebase first
git push origin main

# 6. Clean up
git worktree remove .worktrees/<slug>
git branch -d <slug>

# 7. Mark done
yx done "yak name"
```

## Common Mistakes

- **Working directly on main:** Always create worktree first, set `cwd` to worktree path, never `cwd: "."`
- **Forgetting to mark WIP:** Always `yx start` before creating worktree
- **Not reviewing subagent output:** Review commits and verify acceptance criteria before marking done
- **Working on non-leaf yaks:** Only work on leaves (no children)
- **Skipping context verification:** Verify context exists and is clear before starting
- **Parallel work on dependent yaks:** Check yaks are truly independent; when in doubt, work sequentially
- **Merging without verification:** Always run tests/linting in worktree before merging
- **Prescribing implementation:** Pass yak context; defer to project conventions. Don't tell subagent "use TDD"
- **Modifying main while subagents work:** Once dispatched, you only observe and merge. No editing main

## After Completing Yaks

Once you've completed one or more leaf yaks, present the status:

```
Completed yaks:
- yak name (merged in <commit-hash>)
- another yak (merged in <commit-hash>)

Updated yak tree:
[show yx list output]

Remaining ready leaves:
- other yak (context: brief description)

Next steps:
1. Continue with remaining leaves
2. Use /yx:map to discover more work
3. Use /yx:prepare if any yak needs clearer specs

What would you like to do?
```

## Quick Reference

| Action | Command |
|--------|---------|
| List leaf yaks | `yx list` (look for leaves) |
| Check yak details | `yx show "yak name" --format json` |
| Mark as WIP | `yx start "yak name"` |
| Create worktree | `git worktree add .worktrees/<slug> -b <slug>` |
| Dispatch subagent | `subagent({ agent, task, cwd: ".worktrees/<slug>" })` |
| Verify work | `cd .worktrees/<slug> && npm test` (or project tests) |
| Merge to main | `git merge --ff-only <slug>` |
| Remove worktree | `git worktree remove .worktrees/<slug>` |
| Delete branch | `git branch -d <slug>` |
| Mark done | `yx done "yak name"` |
| Tag needs-human | `yx tag "yak name" @needs-human` |
| Remove tag | `yx tag "yak name" --remove @needs-human` |

## See Also

- **`/yx:map`** — Discover work structure and create yak tree
- **`/yx:prepare`** — Flesh out a yak with detailed specs and acceptance criteria
- **`../docs/yak-workflow.dot`** — High-level workflow showing when to use each skill
- **`worktree-flow.dot`** — Visual diagram of the worktree workflow (if exists)
