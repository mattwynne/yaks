# Release Script with argc

## Goal
Create a release script that packages yx for distribution, preparing for installer testing.

## Requirements
1. Add argc to flake.nix dependencies
2. Create ./bin/dev script using argc
3. Implement `dev release` command that:
   - Zips up bin/yx and completions/ directory
   - Outputs to ./release/ folder (gitignored)
4. This release artifact will be used by install.sh testing
5. Can also be used in GitHub Actions for creating releases

## Implementation Notes
- Use argc for the dev script CLI interface
- Ensure ./release/ is added to .gitignore
- The zip should contain the structure that install.sh expects:
  - bin/yx
  - completions/yx.bash
  - completions/yx.zsh

## Why This Blocks Testing
We need a way to create release artifacts locally so install.sh can be tested against packaged versions (not just files in the repo).
