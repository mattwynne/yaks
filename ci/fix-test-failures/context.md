# Fix CI Test Failures

## Status
Test workflow `.github/workflows/test.yml` is set up and running successfully.
Tests pass locally (108 examples, 0 failures) but fail in CI (108 examples, 16 failures).

## CI Run
https://github.com/mattwynne/yaks/actions/runs/21527033733

## Failures

Most failures are git sync tests failing with initialization issues:
```
error: src refspec main does not match any
error: failed to push some refs to '/run/user/1001/tmp.XYZ'
warning: You appear to have cloned an empty repository.
```

### Failed Tests:
1. spec/features/completions.sh:11 - yx completions lists all yak names
2. spec/features/gitignore_check.sh:3 - shows error when .yaks is not gitignored
3. spec/features/install.sh:3 - installs yx from release zip (expected - no release/yx.zip)
4-10. spec/features/sync.sh - All sync tests (git repo setup issues)
11-12. spec/features/sync_no_pollution.sh - All pollution tests
13. spec/features/sync_push.sh:3 - push to bare repo
14-16. spec/features/sync_worktrees.sh - All worktree sync tests

## Root Cause
The test setup helpers are failing to initialize git repos properly in the CI environment.
Tests use temporary repos with `git init` and try to push to `main` branch before any commits exist.

## Next Steps
- Investigate test setup in spec files (particularly setup_repos function)
- Fix git repo initialization to work in CI environment
- Ensure initial commits exist before push attempts
