# Automated Installer Tests

## Design Overview

Create automated tests for install.sh that run in Docker and verify the installer works end-to-end with functional smoke tests.

## Goals

- Test install.sh in realistic environment (Docker containers)
- Use local release artifacts for fast, deterministic tests
- Verify installer actually works (not just that files get copied)
- Enable non-interactive testing via environment variables

## Design Decisions

### 1. Simplify install.sh with YX_SOURCE

**Current state**: install.sh has branching logic (local vs download)

**New approach**: Always work from a zip file:
- `YX_SOURCE` environment variable points to zip (local path or URL)
- Default: `https://github.com/mattwynne/yaks/releases/download/latest/yx.zip`
- Simpler, more consistent, easier to test

### 2. Environment Variables

Three environment variables for non-interactive mode:
- **YX_SOURCE**: Path or URL to yx.zip (default: latest GitHub release)
- **YX_SHELL_CHOICE**: 1 for zsh, 2 for bash (if unset: prompt)
- **YX_AUTO_COMPLETE**: y or n (if unset: prompt)

### 3. Test Strategy

**Phase 1**: Minimal happy path
- One test: Ubuntu 22.04, bash, no auto-complete
- Uses local release zip (fast, no network)
- Functional verification with smoke tests: `yx --help`, `yx add`, `yx ls`

### 4. Implementation Order (TDD)

1. Update docker_helpers.sh to set YX_SOURCE
2. Run test - expect failure
3. Add zip handling to install.sh
4. Expand test to run smoke tests
5. Add env var support for prompts
6. Remove old conditional logic
7. Test passes

## install.sh Changes

1. Add YX_SOURCE with zip handling (download if URL, copy if local)
2. Extract zip to temp dir
3. Install from extracted files
4. Check env vars before prompting (YX_SHELL_CHOICE, YX_AUTO_COMPLETE)
5. Remove old `if [ -f "bin/yx" ]` conditional

## Test Structure

- `spec/features/install.sh`: Expand to run smoke tests
- `spec/support/docker_helpers.sh`: Update to set YX_SOURCE

## Success Criteria

- `shellspec spec/features/install.sh` passes
- Test runs in Docker with local release zip
- Smoke tests verify yx commands work
- install.sh works for real users (downloads from GitHub latest by default)
