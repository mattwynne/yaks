# Add Shellcheck Linting

Add shellcheck linting to catch common bash scripting errors before they cause issues.

## Goals

- Lint all shell files: bin/yx, spec/*.sh, and any other .sh files
- Use shellcheck's default rules (no .shellcheckrc exceptions)
- Run in two environments: local development and CI
- Fail fast: any shellcheck issue blocks commits/merges

## Philosophy

Fix issues rather than suppress them. Shellcheck catches legitimate bugs.

## Sub-yaks

This yak has children that enforce implementation order:
1. First: setup local dev lint (add to devenv, create dev lint command, fix issues)
2. Then: add github actions workflow (CI integration using dev lint)
