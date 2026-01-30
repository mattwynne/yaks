# Automated Installer Tests

## Current State (2026-01-29)

### What We've Done
- ✓ Brainstormed and validated design
- ✓ Created worktree at `.worktrees/automated-installer-tests`
- ✓ Built release artifact with `dev release` (creates release/yx.zip)
- ✓ Created test infrastructure:
  - `spec/features/install.sh` - ShellSpec test that runs install in Docker
  - `spec/support/docker_helpers.sh` - Helper functions (not currently used)
  - Test runs Docker container with Ubuntu 22.04
  - Sets YX_SOURCE, YX_SHELL_CHOICE, YX_AUTO_COMPLETE env vars
- ✓ Test is in RED state (correct): install.sh currently ignores YX_SOURCE

### Current Challenges
- Test setup took longer than expected (ShellSpec import issues, Docker path issues)
- Need to implement YX_SOURCE support in install.sh (significant refactor)
- Test currently expects failure but install.sh succeeds (uses old local copy logic)

### What We Need To Do Next
(See plan below - using writing-plans skill to detail this)

## Original Design (from brainstorming)

### Goals
- Test install.sh in realistic environment (Docker containers)
- Use local release artifacts for fast, deterministic tests
- Verify installer actually works (not just that files get copied)
- Enable non-interactive testing via environment variables

### Design Decisions

**1. Simplify install.sh with YX_SOURCE**
- Current: Branches on whether bin/yx exists (local vs download)
- New: Always work from zip file via YX_SOURCE env var
- Default: `https://github.com/mattwynne/yaks/releases/download/latest/yx.zip`

**2. Environment Variables**
- YX_SOURCE: Path or URL to yx.zip (default: latest GitHub release)
- YX_SHELL_CHOICE: 1 for zsh, 2 for bash (if unset: prompt)
- YX_AUTO_COMPLETE: y or n (if unset: prompt)

**3. Test Strategy (Phase 1)**
- One test: Ubuntu 22.04, bash, no auto-complete
- Uses local release zip (fast, no network)
- Functional verification with smoke tests

**4. install.sh Changes Needed**
1. Add YX_SOURCE with zip handling (download if URL, copy if local)
2. Extract zip to temp dir
3. Install from extracted files
4. Check env vars before prompting (YX_SHELL_CHOICE, YX_AUTO_COMPLETE)
5. Remove old `if [ -f "bin/yx" ]` conditional

**5. Success Criteria**
- `shellspec spec/features/install.sh` passes
- Test runs in Docker with local release zip
- Smoke tests verify yx commands work
- install.sh works for real users (downloads from GitHub latest by default)
