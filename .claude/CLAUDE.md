# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Yak - DAG-based TODO List CLI

A CLI tool for managing TODO lists as a directed acyclic graph (DAG), designed for teams working on software projects. The name comes from "yak shaving" - when you set out to do task A but discover you need B first, which requires C.

## Core Commands

```bash
# Testing
cargo test --test cucumber --features test-support  # Run Cucumber acceptance tests
shellspec                    # Run ShellSpec tests (tmux smoke, git checks, installer)

# Linting
dev lint                     # Run linting (Rust clippy + rustfmt)

# Quality Checks
dev check                    # Run all checks (tests + lint + audit) - ALWAYS run before committing

# Development
yx add <name>                # Add a yak
yx ls                        # List yaks
yx context <name>            # Edit context (uses $EDITOR or stdin)
yx state <name> <state>      # Set yak state (todo, wip, done)
yx done <name>               # Mark complete
yx rm <name>                 # Remove a yak
yx prune                     # Remove all done yaks
```

Commands like `yx` and `dev` are installed in PATH via direnv.

## Architecture

### Implementation Language
Core implementation is in **Rust** (migrated from bash in Feb 2026 - see ADR 0001).
The compiled binary is at `target/release/yx`, with a symlink at `bin/yx`.

### Hexagonal Architecture (Ports & Adapters)
The codebase uses hexagonal architecture for testability and future extensibility:

**Domain Layer** (`src/domain/`):
- Core entity: `Yak` (name, done status, state, context)

**Application Layer** (`src/application/`):
- Use cases: `AddYak`, `ListYaks`, `DoneYak`, `RemoveYak`, `PruneYaks`,
  `EditContext`, `ShowContext`, `SetState`, `MoveYak`, `SyncYaks`
- Pure business logic, independent of infrastructure

**Ports** (`src/ports/`):
- `StoragePort`: Yak persistence abstraction
- `SyncPort`: Synchronization abstraction
- `LogPort`: Command logging abstraction
- `OutputPort`: User output abstraction

**Adapters** (`src/adapters/`):
- `DirectoryStorage`: File-based storage (`.yaks/` directories)
- `GitRefSync`: Git ref-based sync (future backend)
- `GitLog`: Command logging via git notes
- `ConsoleOutput`: Terminal output with colors

**CLI Entry Point** (`src/main.rs`):
- Command parsing via clap
- Wires together ports and adapters
- Routes commands to use cases

### Target Architecture: CQRS/Event Sourcing
The codebase is evolving toward **CQRS (Command Query Responsibility Segregation)** with **Event Sourcing**. Commands mutate state through aggregates; queries read from projections. Events are the source of truth.

**When making architectural decisions, invoke the `cqrs-event-sourcing` skill** for guidance on aggregate boundaries, event design, read models, policies, and sagas.

### Storage Format
- Uses `YAK_PATH` environment variable (defaults to `.yaks`)
- Each yak is a directory: `$YAK_PATH/<yak-name>/`
- `context.md` holds notes (created empty by default)
- `state` file holds state (todo/wip/done, defaults to "todo")
- The `done` boolean field is derived from state (done = state == "done")
- Directory-based storage allows future backends (git refs) via adapter pattern

### Testing
- **Cucumber acceptance tests** (`features/*.feature`): Primary test framework.
  Runs in two modes via `cargo test --test cucumber --features test-support`:
  - FullStackWorld: spawns yx binary (real integration test)
  - InProcessWorld: calls Rust directly with in-memory adapters (fast)
- **ShellSpec tests** (`tests/shellspec/`): For tests that don't fit Cucumber
  (tmux completion smoke test, git availability check, installer test).
  Run with `shellspec`.
- **Rust unit tests**: Internal logic (`cargo test`)
- **Integration tests**: Exercise use cases with mock adapters

### Mutation Testing
Mutation testing validates test quality by injecting small code
changes (mutants) and checking that tests catch them.

```bash
dev mutate              # Run all (~7 min, 440 mutants)
dev mutate -F 'slug'    # Filter to specific files
```

**When to run:** After adding tests or changing core logic, to
verify your tests actually catch regressions. Runs in CI on
every push.

**Config:** `.cargo/mutants.toml` — excludes infrastructure-only
files (console I/O, git sync, main.rs) that need full-stack
integration tests.

**Reading results:** Check `mutants.out/missed.txt` for mutants
that survived. Each missed mutant is a code change your tests
didn't detect — a potential real bug that could slip through.

## CLI Design Philosophy

**When making changes to the command-line interface, refer to `docs/cli-design-philosophy.md`.**

This guide documents yx's design principles for the CLI, informed by modern best practices (clig.dev, 12 Factor CLI Apps, The Art of Command Line). Key principles:

- **Ergonomics First** - Multi-word names without quotes, short aliases, sensible defaults
- **Human & Machine Output** - Pretty by default, plain format for scripting
- **Clear Feedback** - Actionable error messages that explain what went wrong and how to fix it
- **Composability** - Works well with pipes, stdin, and other Unix tools
- **Speed** - Operations should feel instant (< 100ms)

The guide includes concrete examples, anti-patterns to avoid, and a decision framework for evaluating new features.

## Architecture Decision Records (ADRs)

**ADRs document significant architectural and design decisions.**

### When to Write an ADR

Write an ADR when making decisions that:
- Change the architecture or core design patterns
- Introduce new dependencies or technologies
- Affect multiple components or the public API
- Have long-term maintenance implications
- Involve significant trade-offs between alternatives
- Future maintainers will ask "why did we do it this way?"

**Do NOT write ADRs for:**
- Minor implementation details
- Bug fixes (unless they reveal a design issue)
- Refactoring that preserves behavior
- Configuration changes

### How to Write an ADR

```bash
# Create a new ADR (use quotes for titles with spaces)
adrgen create "Title of the Decision"

# This creates docs/adr/NNNN-title-of-the-decision.md
```

**ADR Workflow:**
1. Identify a significant decision that needs documentation
2. Create the ADR using `adrgen create "<title>"`
3. Edit the generated file in `docs/adr/`:
   - **Context**: Explain the problem and why a decision is needed
   - **Decision**: State what you decided to do
   - **Consequences**: Document trade-offs, what becomes easier/harder
4. Commit the ADR with the related code changes
5. Update status later if needed: `adrgen status <number> <new-status>`

**ADR Location:** `docs/adr/`

**Timing:** Write ADRs during the design/planning phase, before
significant implementation work. If you discover the need for an
ADR during implementation, pause and write it before continuing.

### Linking ADRs to Decisions

ADRs can reference each other:
- `--supersedes <number>`: This ADR replaces an older one
- `--amends <number>`: This ADR modifies an earlier decision

## Development Workflow

**Test-Driven Development (TDD)**:
1. Write ONE failing test (Cucumber scenario or Rust test)
2. Run tests (RED)
3. Implement minimal code to pass (GREEN)
4. Run tests to verify
5. Refactor if needed
6. Run `dev check` to verify all checks pass
7. Commit
8. Repeat

**TRUST THE TESTS**: When tests pass, the feature works. Do NOT run redundant manual verification.

**Incremental approach**: Use the `incremental-tdd` skill for guidance on writing one test at a time.

## Plans

When working on a yak, store implementation plans on the yak's
`plan` field using `yx field <yak-name> plan` (pipe content via stdin).
Read existing plans with `yx field --show <yak-name> plan`.
Do NOT store plans in `docs/superpowers/plans/`.

## CRITICAL: Dogfooding Rule

**NEVER touch the `.yaks` folder in this project!**

We're using yaks to build yaks (dogfooding). The `.yaks` folder contains the actual work tracker for this project.

- **For testing**: Use `YAK_PATH` (tests set this to temp directories)
- **For demos**: Use `YAK_PATH=/tmp/demo-yaks yx <command>`
- **NEVER**: Run `rm -rf .yaks` or modify `.yaks` contents directly

## CRITICAL: Picking Up a Yak

**First action when picking up ANY yak: mark it as WIP.**

```bash
yx state "<yak-name>" wip
```

Do this BEFORE reading context, creating worktrees, or starting any work.
This signals to other agents and to the human what's being worked on.

**If a yak needs requirements fleshed out**, use the `preparing-a-yak` skill first.

**When ready to implement**, use the `yak-worktree-workflow` skill. Follow it exactly.

## Commit Message Policy

**Do NOT include Claude's name or "Co-Authored-By: Claude" in commit messages.**

Commits should be clean and professional without AI attribution.

## Future Vision

The current implementation is Phase 1 (directory-based storage). Future plans include:
- Git ref backend for cross-branch collaboration
- Hierarchy/containment model (yaks contain sub-yaks)
- Team swarming capability (visibility into who's working on what)

Currently out of scope: time tracking, priority levels, rich text, external integrations, auth, cloud sync.
