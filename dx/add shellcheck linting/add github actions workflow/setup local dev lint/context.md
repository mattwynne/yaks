# Setup Local Dev Lint

Add shellcheck to the development environment and create a `dev lint` command.

## Tasks

1. **Add to devenv.nix**: Include shellcheck package so it's available in dev shell
2. **Create dev lint command**: Add to `dev` script
   - Find all shell files: bin/yx and **/*.sh
   - Run shellcheck on each
   - Exit with shellcheck's status code
3. **Fix existing issues**: Run `dev lint` and fix all shellcheck warnings in:
   - bin/yx (main CLI)
   - spec/*.sh (test files)

## File Discovery Pattern

```bash
# Check bin/yx explicitly (no extension)
# Find all .sh files recursively
```

## Output

Shellcheck provides clear error messages:
- File and line number
- Error code (SC####)
- Description and fix suggestion

## Success Criteria

- `dev lint` runs successfully in dev shell
- All existing shell files pass shellcheck with no issues
- Command exits non-zero if problems found
