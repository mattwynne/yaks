# shellcheck shell=bash
Describe 'install.sh'
  It 'installs yx from release zip and runs smoke tests'
    run_install() {
      docker build -t yx-installer-test-base -f "$TEST_PROJECT_DIR/spec/features/Dockerfile.installer-test" "$TEST_PROJECT_DIR" 2>/dev/null

      docker run --rm \
        -v "$TEST_PROJECT_DIR:/workspace" \
        -w /workspace \
        -e YX_SOURCE="/workspace/release/yx.zip" \
        -e YX_SHELL_CHOICE="2" \
        -e YX_AUTO_COMPLETE="n" \
        -e NO_COLOR="1" \
        yx-installer-test-base \
        bash -c '
          ./install.sh
          echo "=== Smoke tests ==="
          yx --help
          cd /tmp
          git init -q .
          git config user.email "test@example.com"
          git config user.name "Test"
          echo ".yaks" > .gitignore
          yx add foo
          yx ls
        '
    }
    When call run_install
    The status should be success
    The entire output should equal "$(cat <<'EOF'
Installing yx (yaks CLI)...

Detected shell: bash
Install completions for:
  1) zsh
  2) bash
Downloading release...
✓ Installed yx to /usr/local/bin/yx
✓ Installed completion to /root/.bash_completion.d/yx

To enable tab completion, add this to /root/.bashrc:

    source /root/.bash_completion.d/yx



Installation complete!
Try: yx --help
=== Smoke tests ===
Usage: yx <command> [arguments]

Commands:
  add <name>                      Add a new yak
  list, ls [--format FMT]         List all yaks
           [--only STATE]
                          --format: Output format
                                    markdown (or md): Checkbox format (default)
                                    plain (or raw): Simple list of names
                          --only: Show only yaks in a specific state
                                  not-done: Show only incomplete yaks
                                  done: Show only completed yaks
  context [--show] <name>         Edit context (uses $EDITOR) or set from stdin
                          --show: Display yak with context
                          --edit: Edit context (default)
  done <name>                     Mark a yak as done
  done --undo <name>              Unmark a yak as done
  rm <name>                       Remove a yak by name
  move <old> <new>                Rename a yak
  mv <old> <new>                  Alias for move
  prune                           Remove all done yaks
  sync                            Push and pull yaks to/from origin via git ref
  completions [cmd]               Output yak names for shell completion
  --help                          Show this help message
- [ ] foo
EOF
)
"
  End
End
