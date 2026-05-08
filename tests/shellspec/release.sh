# shellcheck shell=bash
# shellcheck disable=SC2034 # ShellSpec reads assertion variables dynamically.

Describe 'bin/release'
  setup_release_repo() {
    RELEASE_REPO=$(mktemp -d)
    RELEASE_ORIGIN=$(mktemp -d)
    git init --bare --quiet "$RELEASE_ORIGIN"
    git -C "$RELEASE_REPO" init --initial-branch=main --quiet
    git -C "$RELEASE_REPO" config user.email test@example.com
    git -C "$RELEASE_REPO" config user.name 'Test User'

    mkdir -p "$RELEASE_REPO/src"
    cat > "$RELEASE_REPO/Cargo.toml" <<'EOF'
[package]
name = "yx"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
EOF
    echo 'pub fn ok() {}' > "$RELEASE_REPO/src/lib.rs"
    cat > "$RELEASE_REPO/CHANGELOG.md" <<'EOF'
# Changelog

## [Unreleased]

- Something changed.
EOF
    (cd "$RELEASE_REPO" && cargo generate-lockfile >/dev/null)
    git -C "$RELEASE_REPO" add Cargo.toml Cargo.lock CHANGELOG.md src/lib.rs
    git -C "$RELEASE_REPO" commit --quiet -m 'Initial commit'
    git -C "$RELEASE_REPO" remote add origin "$RELEASE_ORIGIN"
    git -C "$RELEASE_REPO" push --quiet -u origin main
  }

  cleanup_release_repo() {
    rm -rf "$RELEASE_REPO" "$RELEASE_ORIGIN"
  }

  BeforeEach 'setup_release_repo'
  AfterEach 'cleanup_release_repo'

  It 'prepares a release commit and annotated tag without pushing'
    When run sh -c "cd '$RELEASE_REPO' && YX_RELEASE_CHECK_COMMAND=true YX_RELEASE_DATE=2026-05-07 '$TEST_PROJECT_DIR/bin/release' 0.2.0"
    The status should be success
    The output should include 'Prepared release v0.2.0'
    The output should include 'git push origin main && git push origin v0.2.0'
    The contents of file "$RELEASE_REPO/Cargo.toml" should include 'version = "0.2.0"'
    The contents of file "$RELEASE_REPO/Cargo.lock" should include 'version = "0.2.0"'
    The contents of file "$RELEASE_REPO/CHANGELOG.md" should include '## [Unreleased]'
    The contents of file "$RELEASE_REPO/CHANGELOG.md" should include '## [0.2.0] - 2026-05-07'
    commit_subject=$(git -C "$RELEASE_REPO" log -1 --pretty=%s)
    tag_name=$(git -C "$RELEASE_REPO" tag -l v0.2.0)
    if git -C "$RELEASE_ORIGIN" rev-parse --verify refs/tags/v0.2.0 >/dev/null 2>&1; then
      tag_pushed_status=0
    else
      tag_pushed_status=1
    fi
    The variable commit_subject should eq 'Release v0.2.0'
    The variable tag_name should eq 'v0.2.0'
    The variable tag_pushed_status should eq 1
  End

  It 'refuses to release from a non-main branch'
    git -C "$RELEASE_REPO" switch --quiet -c feature
    When run sh -c "cd '$RELEASE_REPO' && YX_RELEASE_CHECK_COMMAND=true '$TEST_PROJECT_DIR/bin/release' 0.2.0"
    The status should be failure
    The error should include 'must be on main'
  End

  It 'refuses to release with a dirty working tree'
    echo dirty >> "$RELEASE_REPO/CHANGELOG.md"
    When run sh -c "cd '$RELEASE_REPO' && YX_RELEASE_CHECK_COMMAND=true '$TEST_PROJECT_DIR/bin/release' 0.2.0"
    The status should be failure
    The error should include 'working tree must be clean'
  End
End
