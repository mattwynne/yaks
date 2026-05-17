This file provides guidance to Agents working in this repository.

We use yaks for task tasking. See !yx help

See [CONTRIBUTING](./CONTRIBUTING.md) for dev tooling and commands.

## Architecture

Hexagonal architecture with CQRS and Event Sourcing. Commands flow
through use cases to the `YakMap` aggregate, which emits domain events.
Events are persisted to git refs and projected to the `.yaks/`
directory-based read model.

**See [`src/README.md`](src/README.md)** for the full architecture
guide: layers, key types, ports, adapters, and how commands flow.

**See [`docs/adr/`](docs/adr/README.md)** for Architecture Decision
Records explaining why the design is the way it is.

When making architectural decisions, invoke the `cqrs-event-sourcing`
skill for guidance on aggregate boundaries, event design, read models,
policies, and sagas.

## Testing layers

- **Cucumber acceptance tests** (`features/*.feature`): For executable
  specifications, agreed with human partner. Dual-mode execution via
  `cargo test --test cucumber --features test-support`:
  - FullStackWorld: spawns yx binary (real integration test)
  - InProcessWorld: calls Rust directly with in-memory adapters (fast)
- **ShellSpec tests** (`tests/shellspec/`): For tests that don't fit
  Cucumber (tmux smoke, git checks, installer). Run with `shellspec`.
- **Rust unit tests**: Internal logic (`cargo test`)

### Mutation Testing

Validates test quality by injecting code changes and checking tests
catch them.

```bash
dev mutate-diff         # Fast: only mutants in your changes (~seconds)
dev mutate              # Full run (~7 min, ~400 mutants)
dev mutate -F 'slug'    # Full run filtered to specific files
dev mutate-sync         # Sync missed mutants to yaks
```

**Daily workflow:** Use `dev mutate-diff` (alias `dev md`) while coding.

**After a full run:** `dev mutate-sync` (alias `dev ms`) creates yaks
for missed mutants. Then `yx sync` to share results.

**Config:** `.cargo/mutants.toml` excludes infrastructure-only files.

**Triage:** Leave real test gaps as `todo` yaks. For acceptable misses,
add to `exclude_globs` in `.cargo/mutants.toml`.

## CLI Design Philosophy

**See `docs/cli-design-philosophy.md`** when making changes to the CLI.

Key principles: ergonomics first, human & machine output, clear
feedback, composability, speed (< 100ms).

## Development Workflow

**All work must be tracked in a yak.** Before starting any non-trivial
work (bug fixes, refactoring, new features, infrastructure changes),
check whether it's modelled in a yak. If not, challenge the human:

> "This work isn't tracked in a yak. Should we create one before
> proceeding? e.g. `yx add <suggested name> --under <parent>`"

This keeps the yak tree honest and avoids untracked work drifting
off course. The only exceptions are trivial one-line fixes.

**Test-Driven Development (TDD)**:
1. Write ONE failing test (Cucumber scenario or Rust test)
2. Run tests (RED)
3. Implement minimal code to pass (GREEN)
4. Run tests to verify
5. Refactor. Mercilessly. Simplicity is your friend. Loop multiple
   times here until the code is exemplary
6. Run `dev check` to verify all checks pass
7. Commit
8. Repeat

**TRUST THE TESTS**: When tests pass, the feature works. Do NOT run
redundant manual verification.

## Don't interfere with this project's yak map

**NEVER touch the `.yaks` folder in this project!**

We're using yaks to build yaks. The `.yaks` folder
contains the actual work tracker for this project.

- **For testing**: Use `YX_ROOT` (tests set this to temp repo directories)
- **For demos**: Use `YX_ROOT=/tmp/demo-repo yx <command>` (where demo-repo is a git repo)
- **NEVER**: Run `rm -rf .yaks` or modify `.yaks` contents directly

## ADR Policy

**Never modify existing accepted ADRs.** Write new ADRs that supersede
them instead.

## Completing a Yak

**Merge early and often.** When work is done, merge to main
immediately. Do not wait for permission. This is trunk-based
development — branch age is the enemy.

**Always use `dev merge <branch>` to merge.** This rebases the
branch onto main, runs `dev check`, and only fast-forwards main
if all checks pass. Never use `git merge` directly.

After merging:
1. `yx done "<yak-name>"` — mark the yak complete
2. Clean up the worktree and branch

## Commit Message Policy

**Do NOT include Claude's name or "Co-Authored-By: Claude" in commit
messages.**

Commits should be clean and professional without AI attribution.
