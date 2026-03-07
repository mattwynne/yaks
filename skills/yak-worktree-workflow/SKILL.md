---
name: yak-worktree-workflow
description: Use when starting work on a yak - sets up an isolated git worktree, reads yak context, and guides the full cycle from claiming through merge and cleanup
---

# Yak Worktree Workflow

Use the `start_yak`, `show_yak`, `merge_yak`, and subagent tools to
work on a yak in an isolated worktree.

## The Workflow

### 1. Start the yak

Call the `start_yak` tool with the yak name. This:
- Marks it as wip
- Creates a git worktree in `.worktrees/<yak-id>`
- Returns the worktree path and branch name

### 2. Read the yak context

Call `show_yak` to get the full details including context.

**If the context is missing, empty, or too vague — STOP and ask
the user.** Do not proceed without clear requirements. Ask
specific questions:
- "The context doesn't specify X. Should I...?"
- "There are multiple approaches. Do you prefer A or B?"

### 3. Delegate to a worker subagent

Use the `subagent` tool to delegate implementation:

```
subagent(
  agent: "worker",
  cwd: "<worktree-path>",
  task: "<derived from yak context>"
)
```

The task should include:
- What to build (from the yak context)
- Key constraints or acceptance criteria
- A reminder to commit changes before finishing

### 4. Merge

Call the `merge_yak` tool with the branch name. This:
- Runs `dev check` (tests, lint, complexity, mutation tests)
- Rebases onto main
- Fast-forward merges
- Removes the worktree and branch

If merge fails, read the output, fix issues via another subagent
round in the worktree, and try again.

### 5. Mark done

Use bash: `yx done "<yak name>"`

## Key Principles

- **Verify context before starting work.** The yak name alone
  doesn't tell you what to build.
- **Merge as soon as tests pass.** This is trunk-based development.
  Branch age is the enemy.
- **Summarise what you did** when reporting back to the user.
