# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Yak - DAG-based TODO List CLI

A CLI tool for managing TODO lists as a directed acyclic graph (DAG), designed for teams working on software projects. The name comes from "yak shaving" - when you set out to do task A but discover you need B first, which requires C.

## Core Commands

```bash
# Testing
shellspec                    # Run all tests
shellspec spec/list.sh       # Run specific test file

# Development
yx add <name>                # Add a yak
yx ls                        # List yaks
yx context <name>            # Edit context (uses $EDITOR or stdin)
yx done <name>               # Mark complete
yx rm <name>                 # Remove a yak
yx prune                     # Remove all done yaks
```

The command is `yx` (installed in PATH via direnv), not `./yx`.

## Architecture

### Single-File CLI
All logic is in `bin/yx` - a ~240 line bash script organized into functions:
- Command routing via case statement (line 199+)
- Each command is a function (list_yaks, add_yak, done_yak, etc.)
- Storage: `.yaks/<yak-name>/` directories with optional `done` marker and `context.md`

### Storage Pattern
- Uses `YAK_PATH` environment variable (defaults to `.yaks`)
- Each yak is a directory: `$YAK_DIR/<yak-name>/`
- `done` file marks completion
- `context.md` holds additional notes
- Adapter pattern allows future backends (git refs planned)

### Testing
- Framework: ShellSpec
- Pattern: Each command has its own spec file (spec/add.sh, spec/list.sh, etc.)
- Tests use `YAK_PATH=$(mktemp -d)` for isolation
- Configuration: `.shellspec` sets format, pattern, and shell

## Development Workflow

**Test-Driven Development (TDD)**:
1. Write ONE failing test
2. Run `shellspec` (RED)
3. Implement minimal code to pass (GREEN)
4. Run `shellspec` to verify
5. Refactor if needed
6. Commit
7. Repeat

**TRUST THE TESTS**: When tests pass, the feature works. Do NOT run redundant manual verification.

**Incremental approach**: Use the `incremental-tdd` skill for guidance on writing one test at a time.

## CRITICAL: Dogfooding Rule

**NEVER touch the `.yaks` folder in this project!**

We're using yaks to build yaks (dogfooding). The `.yaks` folder contains the actual work tracker for this project.

- **For testing**: Use `YAK_PATH` (tests set this to temp directories)
- **For demos**: Use `YAK_PATH=/tmp/demo-yaks yx <command>`
- **NEVER**: Run `rm -rf .yaks` or modify `.yaks` contents directly

## CRITICAL: Picking Up a Yak

**ALWAYS use a worktree when working on a yak. NEVER work directly on main.**

When the user asks you to pick up a yak, follow this workflow EXACTLY:

### Yak Workflow Checklist

- [ ] **Create worktree**: Use `git worktree add .worktrees/<branch-name> -b <branch-name>`
- [ ] **Read yak context**: Run `yx context --show <yak-name>` to understand the task
- [ ] **Ask for clarification**: If context is empty or unclear, ask the user - do not assume
- [ ] **Do the work**: In the worktree, run tests, make changes, commit
- [ ] **Verify tests pass**: Run `shellspec` to ensure all tests are green
- [ ] **Switch to main**: `cd` back to the main repository directory
- [ ] **Merge to main**: `git merge --no-ff <branch-name> -m "Merge <branch>: <description>"`
- [ ] **Delete worktree**: `git worktree remove .worktrees/<branch-name>`
- [ ] **Delete branch**: `git branch -d <branch-name>`
- [ ] **Mark yak done**: Run `yx done <yak-name>` ONLY after merge and cleanup

**CRITICAL**: NEVER mark a yak as done until AFTER merging to main and deleting the worktree. The order matters.

This applies to ALL yak work, regardless of how "simple" the change appears. No exceptions.

**Why worktrees matter:**
- **Isolation**: Keep main clean while working
- **Safety**: Test changes without affecting the main branch
- **Dogfooding**: We use yaks to build yaks - follow the same workflow
- **Collaboration**: Multiple agents/developers can work on different yaks simultaneously

**Worktree directory location:**
Use `.worktrees/` for all git worktrees in this project. This directory is ignored in `.gitignore`.

## Commit Message Policy

**Do NOT include Claude's name or "Co-Authored-By: Claude" in commit messages.**

Commits should be clean and professional without AI attribution.

## Future Vision

The current implementation is Phase 1 (directory-based storage). Future plans include:
- Git ref backend for cross-branch collaboration
- Hierarchy/containment model (yaks contain sub-yaks)
- Team swarming capability (visibility into who's working on what)

Currently out of scope: time tracking, priority levels, rich text, external integrations, auth, cloud sync.
