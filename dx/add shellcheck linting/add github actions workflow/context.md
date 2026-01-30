# Add GitHub Actions Workflow

Create a CI workflow that runs shellcheck on every push to catch issues before merge.

## Implementation

Create `.github/workflows/lint.yml`:
- Run on every push
- Use existing nix setup (cachix/install-nix-action)
- Run `dev lint` command
- Fail build if any shellcheck issues found

## Why separate workflow

- Runs independently of release workflow
- Faster feedback on PRs
- Clear status: lint check shows separately from release
- Can configure to run on pull requests

## Prerequisites

This yak depends on "setup local dev lint" being complete first. The CI workflow uses the `dev lint` command, so that must exist and work correctly before adding CI.
