# shellcheck shell=bash
Describe 'install.sh'
  docker_unavailable() { ! docker info >/dev/null 2>&1; }

  It 'installs yx from release zip and runs smoke tests'
    Skip if "release not present: run \`dev release-linux\`" test ! -f "$TEST_PROJECT_DIR/result-linux/yx-linux.zip"
    Skip if "docker not available" docker_unavailable
    run_install() {
      # Copy zip to a temp location for Docker
      temp_zip=$(mktemp)
      cp "$TEST_PROJECT_DIR/result-linux/yx-linux.zip" "$temp_zip"

      docker build -t yx-installer-test-base -f "$TEST_PROJECT_DIR/tests/shellspec/Dockerfile.installer-test" "$TEST_PROJECT_DIR" 2>/dev/null

      docker run --rm \
        -v "$TEST_PROJECT_DIR:/workspace" \
        -v "$temp_zip:/tmp/yx.zip" \
        -w /workspace \
        -e YX_SOURCE="/tmp/yx.zip" \
        -e YX_SHELL_CHOICE="2" \
        -e YX_AUTO_COMPLETE="n" \
        -e NO_COLOR="1" \
        yx-installer-test-base \
        bash -c '
          ./install.sh
          echo "=== Smoke tests ==="
          cd /tmp
          git init -q .
          git config user.email "test@example.com"
          git config user.name "Test"
          echo ".yaks" > .gitignore
          yx add foo >/dev/null
          yx ls --format markdown
        '

      rm -f "$temp_zip"
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
✓ Installed yx to /usr/local/lib/yaks
✓ Linked /usr/local/bin/yx -> /usr/local/lib/yaks/bin/yx
✓ Installed completion to /root/.bash_completion.d/yx

To enable tab completion, add this to /root/.bashrc:

    source /root/.bash_completion.d/yx



Installation complete!
Try: yx --help
=== Smoke tests ===
- [todo] foo
EOF
)
"
  End
End
