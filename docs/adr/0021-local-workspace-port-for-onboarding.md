# 21. LocalWorkspacePort for onboarding

Date: 2026-03-09

## Status

accepted

## Context

When a user runs `yx` for the first time in a repo, the `.yaks`
directory needs to be added to `.gitignore` to prevent git from
tracking internal yaks state.

Previously, if `.yaks` was not gitignored, `yx` would fail with
a hard error. The user had to manually add `.yaks` to `.gitignore`
and figure out the setup themselves. This created a poor first-run
experience — users would encounter an error before they could do
anything useful with the tool.

We wanted a friendly onboarding experience that automatically
handles this setup. The system should:
1. Detect when `.yaks` is not gitignored
2. In interactive mode, welcome the user and offer to fix it
3. In non-interactive mode, fail with a clear error message
4. Modify `.gitignore` and optionally commit the change

However, these operations — checking gitignore status via
`git check-ignore`, modifying `.gitignore`, and committing changes
— are infrastructure concerns. The codebase uses hexagonal
architecture (ADR 0001) where domain logic communicates with
infrastructure through port traits. According to ADR 0008 (Keep
main.rs thin), they should not live in `main.rs`. According to
ADR 0013 (Every CLI command is a UseCase), the onboarding logic
belongs in a use case, not called directly from the application
layer.

The challenge: we need to check and potentially modify `.gitignore`
before any command runs, but we need to do so through the ports
and adapters pattern to keep the logic testable and maintain
architectural boundaries.

## Decision

Introduce **`LocalWorkspacePort`** trait in the domain layer as
the port for local git and filesystem workspace operations.

The port provides three operations:
- `is_yaks_gitignored()` — check if `.yaks` is already gitignored
- `add_yaks_to_gitignore()` — append `.yaks` to `.gitignore`
- `commit_gitignore()` — stage and commit `.gitignore` with a
  standard message

Create **`EnsureGitignore`** use case that orchestrates the
onboarding flow:
1. Check if `.yaks` is gitignored via `LocalWorkspacePort`
2. If not, check `InputPort::is_interactive()`
3. In interactive mode: display welcome message, prompt to add
   `.yaks`, prompt to commit
4. In non-interactive mode: fail with clear error
5. Use `LocalWorkspacePort::add_yaks_to_gitignore()` and
   `commit_gitignore()` to make changes

The use case runs before every command in `main.rs`:
```rust
// Ensure .yaks is gitignored (runs before any other command)
app.handle(EnsureGitignore::new())?;

// Then route to the actual command
route_command(cli.command, &mut app, stdin)
```

`LocalWorkspace` adapter provides the concrete implementation:
- Uses `git check-ignore` to test if `.yaks` is ignored
- Modifies `.gitignore` file on disk
- Executes `git add .gitignore && git commit -m "Add .yaks to
  .gitignore"`

## Consequences

### What becomes easier

- **Testable onboarding**: The entire onboarding flow can be
  tested with mocked ports. Tests can verify the prompts, the
  decisions, and the gitignore modifications without touching
  the real filesystem or git.
- **Friendly first-run**: New users get a welcoming interactive
  prompt that walks them through setup instead of an error.
- **Clean architecture**: `main.rs` stays thin — it just wires
  the `LocalWorkspace` adapter and calls `app.handle()`. No git
  or filesystem logic leaks into the entry point.
- **Future workspace operations**: `LocalWorkspacePort` provides
  a natural home for other workspace-level concerns (worktree
  management, git hooks, repository validation, etc.).

### What becomes harder

- **Performance overhead**: Every command invocation pays the cost
  of checking gitignore status, even when `.yaks` is already
  configured. However, this is a cheap git operation (just
  `git check-ignore .yaks`) and adds negligible latency.
- **Another port to mock**: Tests that construct an `Application`
  now need to provide a `LocalWorkspacePort` implementation. For
  most tests, the in-memory mock can just return `Ok(true)` for
  `is_yaks_gitignored()` to skip onboarding.

### What remains open

- **Scope of LocalWorkspacePort**: Currently it only handles
  gitignore operations. As other workspace-level concerns arise
  (git hooks, worktree creation cleanup, repo health checks), we
  may extend this port or split it into more focused traits.
- **Caching gitignore check**: We could cache the result of
  `is_yaks_gitignored()` for the lifetime of the `Application`
  to avoid repeated git calls. Currently not implemented because
  the check is cheap and keeps the code simple.
