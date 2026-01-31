# Fix gitignore check test

## Problem
Test at spec/features/gitignore_check.sh:3 is failing because `git check-ignore` reads from global/system gitignore files, not just the local .gitignore.

If `.yaks` is in the user's global gitignore (~/.config/git/ignore or core.excludesFile), the test thinks it's gitignored even when the local .gitignore doesn't have it.

## Solution
Isolate git from global/system config in the test using environment variables:
- `GIT_CONFIG_GLOBAL=/dev/null` - disables global config
- `GIT_CONFIG_NOSYSTEM=1` - disables system config

## Acceptance Criteria
- The gitignore check test passes (spec/features/gitignore_check.sh:3)
- Test result shows 107/108 tests passing (only install.sh test remains failing)
