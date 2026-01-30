Now that we have a proper release script, I think we can start shipping argc in our release and depending on it from the bash script, which will make argument parsing a lot simpler.

## Acceptance Criteria

We are done when:
1. The bin/yx script uses argc for command-line argument parsing
2. All shellspec tests pass
3. The installer test passes
4. argc binary is shipped in the release zip

## Implementation Approach

### Phase 1: Include argc in release
- Modify flake.nix to copy argc binary into release zip alongside bin/yx
- Update nativeBuildInputs to include pkgs.argc
- Adjust buildPhase to copy ${pkgs.argc}/bin/argc into release-bundle/bin/
- Verify with: unzip -l result/yx.zip | grep argc

### Phase 2: Add argc to bin/yx header
- Add argc evaluation at top of bin/yx script
- Detect argc from same directory (for release) or fall back to system PATH
- Use: eval "$("$ARGC_BIN" --argc-eval "$0" "$@")"
- Keep all existing code below this for now

### Phase 3: Convert commands to argc format
- Add argc comment declarations (@cmd, @arg, @option, @flag, @alias)
- Start with simple commands: add, rm, prune, sync
- Then tackle list with its --format and --only options
- Handle done with its --undo and --recursive flags
- Remove parse_format_option and parse_only_option functions
- Remove manual case statement once all commands converted
- Argc will handle command routing via the comment declarations

### Phase 4: Update installer
- Modify install.sh to install argc binary alongside yx
- Add chmod +x for argc binary
- Update installer test expectations if needed

### Phase 5: Verification
- Run full shellspec test suite
- Run installer test (spec/features/install.sh)
- Manual smoke tests

## Technical Details

Argc uses comment-based declarations:
```bash
# @cmd Add a new yak
# @arg name* The yak name
add() {
  # argc puts args in argc_name variables
  add_yak "$argc_name"
}
```

## Resources

- Argc GitHub: https://github.com/sigoden/argc
- Already in devenv.nix on line 14
- Argc is in nixpkgs: pkgs.argc
