# shellcheck shell=bash

Describe 'check-release'
  setup_release_repo() {
    RELEASE_REPO=$(mktemp -d)
    export RELEASE_REPO
    setup_test_repo "$RELEASE_REPO"
    mkdir -p "$RELEASE_REPO/bin" "$RELEASE_REPO/test-bin"
    ln -s "$TEST_PROJECT_DIR/bin/check-release" "$RELEASE_REPO/bin/check-release"
    ln -s "$TEST_PROJECT_DIR/bin/changelog-section" "$RELEASE_REPO/bin/changelog-section"

    cat > "$RELEASE_REPO/Cargo.toml" <<'EOF'
[package]
name = "yx"
version = "1.2.3"
edition = "2021"
EOF
    : > "$RELEASE_REPO/Cargo.lock"
    cat > "$RELEASE_REPO/CHANGELOG.md" <<'EOF'
# Changelog

## [1.2.3] - 2026-05-07

- Release notes for 1.2.3.

## [1.2.2] - 2026-05-06

- Previous release.
EOF
    cat > "$RELEASE_REPO/test-bin/cargo" <<'EOF'
#!/usr/bin/env bash
if [ "$1" = "metadata" ] && [ "$2" = "--locked" ]; then
  exit 0
fi
echo "unexpected cargo args: $*" >&2
exit 1
EOF
    chmod +x "$RELEASE_REPO/test-bin/cargo"
    cat > "$RELEASE_REPO/test-bin/yx" <<'EOF'
#!/usr/bin/env bash
echo "yx 1.2.3"
EOF
    chmod +x "$RELEASE_REPO/test-bin/yx"
  }

  cleanup_release_repo() {
    rm -rf "$RELEASE_REPO"
    unset RELEASE_REPO
  }

  BeforeEach 'setup_release_repo'
  AfterEach 'cleanup_release_repo'

  It 'passes when release metadata is consistent'
    When run sh -c "cd '$RELEASE_REPO' && PATH='$RELEASE_REPO/test-bin':\$PATH bin/check-release 1.2.3"
    The status should be success
    The output should include "Release 1.2.3 is ready"
  End

  It 'accepts a v-prefixed version argument'
    When run sh -c "cd '$RELEASE_REPO' && PATH='$RELEASE_REPO/test-bin':\$PATH bin/check-release v1.2.3"
    The status should be success
    The output should include "Cargo.toml version matches"
  End

  It 'rejects versions that are not SemVer-like'
    When run sh -c "cd '$RELEASE_REPO' && PATH='$RELEASE_REPO/test-bin':\$PATH bin/check-release 1.2"
    The status should be failure
    The error should include "Version must look like SemVer"
  End

  It 'fails when Cargo.toml version does not match'
    When run sh -c "cd '$RELEASE_REPO' && PATH='$RELEASE_REPO/test-bin':\$PATH bin/check-release 2.0.0"
    The status should be failure
    The output should include "Version format: 2.0.0"
    The error should include "Cargo.toml package.version is 1.2.3, expected 2.0.0"
  End

  It 'fails when the changelog section is empty'
    cat > "$RELEASE_REPO/CHANGELOG.md" <<'EOF'
# Changelog

## [1.2.3] - 2026-05-07

## [1.2.2] - 2026-05-06

- Previous release.
EOF
    When run sh -c "cd '$RELEASE_REPO' && PATH='$RELEASE_REPO/test-bin':\$PATH bin/check-release 1.2.3"
    The status should be failure
    The output should include "Cargo.lock is up to date"
    The error should include "changelog section for version 1.2.3 is empty or only headings"
  End

  It 'fails when yx --version does not match an available binary'
    cat > "$RELEASE_REPO/test-bin/yx" <<'EOF'
#!/usr/bin/env bash
echo "yx 9.9.9"
EOF
    chmod +x "$RELEASE_REPO/test-bin/yx"
    When run sh -c "cd '$RELEASE_REPO' && PATH='$RELEASE_REPO/test-bin':\$PATH bin/check-release 1.2.3"
    The status should be failure
    The output should include "CHANGELOG.md has a non-empty 1.2.3 section"
    The error should include "does not include 1.2.3"
  End
End

Describe 'changelog-section'
  It 'prints only the requested changelog section body'
    tmp=$(mktemp -d)
    cat > "$tmp/CHANGELOG.md" <<'EOF'
# Changelog

## [1.2.3] - 2026-05-07

- Wanted.

## [1.2.2]

- Not wanted.
EOF
    When run "$TEST_PROJECT_DIR/bin/changelog-section" 1.2.3 "$tmp/CHANGELOG.md"
    The output should include "Wanted"
    The output should not include "Not wanted"
    rm -rf "$tmp"
  End
End
