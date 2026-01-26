# Yak Worktree Workflow

**Working on yaks in isolation using git worktrees**

## Purpose

When multiple Claude agents work on the same yaks codebase, use git worktrees to:
- Work independently without interfering with other agents
- Keep each yak's work isolated on its own branch
- Enable parallel work on different yaks

## The Workflow

### 1. Check Available Yaks

```bash
yx list
```

Ask the user which yak to work on, or let them pick one for you.

### 2. Read the Yak Context

**CRITICAL**: Always read the yak's context before starting work.

```bash
cd /path/to/main/repo  # Go to main repo, not worktree
yx context --show "yak name here"
```

The context contains:
- Requirements and acceptance criteria
- Design decisions
- Important constraints

**Never skip this step.** The yak name alone doesn't tell you what to build.

### 2a. Verify Context is Sufficient

**CRITICAL**: If the context is missing, empty, or too vague, **STOP and ASK THE USER** for clarification.

Do NOT proceed if:
- The context file is empty or missing
- Requirements are unclear or ambiguous
- You're unsure what success looks like
- Multiple approaches are possible and no preference is stated

Ask specific questions:
- "The context doesn't specify X. Should I...?"
- "I see the goal is Y, but how should Z work?"
- "There are multiple ways to do this. Do you prefer A or B?"

**Only create the worktree after you have sufficient context to proceed.**

### 3. Create a Worktree

Create a worktree in `.claude/worktrees/` with a descriptive branch name:

```bash
mkdir -p .claude/worktrees
git worktree add .claude/worktrees/descriptive-name -b descriptive-name
```

Example:
```bash
git worktree add .claude/worktrees/sort-ls-results -b sort-ls-results
```

### 4. Switch to the Worktree

```bash
cd .claude/worktrees/descriptive-name
```

All your work happens here. You're now on an isolated branch.

### 5. Do the Work

Follow your normal development process:
- Write tests (use incremental-tdd skill if applicable)
- Implement the feature
- Run tests to verify
- Commit your changes

The commits stay on your feature branch, isolated from main.

### 6. Demo Your Work

**CRITICAL**: Before merging, demonstrate your work to the user.

Show:
- What you implemented
- Test results (prove all tests pass)
- Example usage or output
- Any design decisions you made

This lets the user:
- Verify the work meets requirements
- Provide feedback before merging
- Understand what changed

**Wait for user approval before proceeding to merge.**

### 7. Merge Back to Main

After the user approves the demo, return to the main repo and merge:

```bash
cd /path/to/main/repo  # Back to main repo
git merge descriptive-name
```

### 8. Mark the Yak Done

```bash
yx done "yak name here"
```

Use the exact yak name (with spaces if needed).

### 9. Clean Up

Remove the worktree and delete the branch:

```bash
git worktree remove .claude/worktrees/descriptive-name
git branch -d descriptive-name
```

## Key Principles

### Always Read Context First

The yak context is the source of truth. Don't guess requirements from the yak name.

**If context is insufficient, ask the user for clarification BEFORE creating the worktree.** Don't make assumptions or guess at requirements. It's better to ask than to build the wrong thing.

### Demo Before Merging

Always demonstrate your work before merging to main. Show test results, example usage, and explain your approach. Wait for user approval before proceeding with the merge.

This prevents unwanted changes from reaching main and gives the user a chance to provide feedback.

### Use .claude/worktrees/

Keep all worktrees in `.claude/worktrees/` for consistency and easy cleanup.

### Branch Names

Use descriptive, kebab-case branch names that match the worktree directory:
- `sort-ls-results`
- `implement-claim-command`
- `refactor-bash-spaghetti`

### Never Touch .yaks in This Project

**DOGFOODING WARNING**: This project uses yaks to track its own development.

- Never modify `.yaks/` directly
- Tests use `YAK_PATH` env var to use temp directories
- For demos, set `YAK_PATH=/tmp/demo-yaks` or similar

The `.yaks` folder contains real project data - treat it as sacred.

## Troubleshooting

### Yak not found in worktree

The `.yaks` directory isn't copied to the worktree. Always go back to the main repo to run `yx` commands against the actual yak list:

```bash
cd ../../..  # Back to main repo
yx context --show "yak name"
yx done "yak name"
```

### Multiple agents working on same yak

If another agent is already working on a yak, pick a different one. The worktree approach keeps work isolated, but merging conflicts is still annoying.

### Forgot to read context

Stop. Go back to main repo. Read the context. Adjust your approach if needed.

### Context is empty or vague

Ask the user for clarification before proceeding. Examples:
- "The context for 'refactor that bash spagetti' is empty. What specific issues should I address?"
- "The context says 'add claim command' but doesn't specify behavior. Should users be able to claim multiple yaks?"
- "Should the 'edit' command open an editor or accept text from stdin?"

### User wants changes after demo

If the user requests changes during the demo:
1. Go back to the worktree: `cd .claude/worktrees/descriptive-name`
2. Make the requested changes
3. Commit them
4. Demo again
5. Only merge after approval

The worktree keeps your changes isolated, so iteration is safe.

## Why This Works

- **Isolation**: Each worktree is a separate working directory with its own branch
- **Parallel work**: Multiple agents can work on different yaks simultaneously
- **Clean history**: Feature branches keep the work organized
- **No interference**: Your changes don't affect other agents until you merge

## Integration with Other Skills

- **incremental-tdd**: Use TDD workflow within your worktree
- **discovery-tree-workflow**: Discover sub-yaks while working, add them to the main repo

## Example Session

```bash
# User: "Pick up the sort yak"

# 1. Check what's available
yx list

# 2. Read context (from main repo)
yx context --show "sort ls results somehow"
# Output: "Sort by done first, then by creation date..."

# 3. Create worktree
git worktree add .claude/worktrees/sort-ls-results -b sort-ls-results

# 4. Switch to worktree
cd .claude/worktrees/sort-ls-results

# 5. Do the work (write tests, implement, commit)
# ... work happens here ...

# 6. Demo the work
# Show test results, example usage, explain changes
# Wait for user approval

# 7. Return to main and merge (after approval)
cd ../../..
git merge sort-ls-results

# 8. Mark done
yx done "sort ls results somehow"

# 9. Cleanup
git worktree remove .claude/worktrees/sort-ls-results
git branch -d sort-ls-results
```

## When NOT to Use Worktrees

- Single-agent development (just work on main)
- Quick fixes that won't conflict
- User explicitly asks to work directly on main

For multi-agent scenarios or parallel work, worktrees are essential.
