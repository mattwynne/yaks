# shellcheck shell=bash

Describe '.githooks/verify-checks content-addressed marker'
  setup_repo_with_dev_scripts() {
    repo=$(mktemp -d)
    export repo
    git -C "$repo" init --initial-branch=main --quiet
    git -C "$repo" config user.email test@example.com
    git -C "$repo" config user.name "Test User"
    mkdir -p "$repo/bin" "$repo/.githooks" "$repo/src"
    cp "$TEST_PROJECT_DIR/bin/dev" "$repo/bin/dev"
    cp "$TEST_PROJECT_DIR/.githooks/verify-checks" "$repo/.githooks/verify-checks"
    chmod +x "$repo/bin/dev" "$repo/.githooks/verify-checks"
    printf '[package]\nname = "example"\nversion = "0.1.0"\n' > "$repo/Cargo.toml"
    printf '# shellspec config\n' > "$repo/.shellspec"
    printf 'version = 3\n' > "$repo/Cargo.lock"
    printf 'fn main() {}\n' > "$repo/src/main.rs"
    git -C "$repo" add .
    git -C "$repo" commit --quiet -m base
  }

  cleanup_repo() {
    rm -rf "$repo"
  }

  BeforeEach 'setup_repo_with_dev_scripts'
  AfterEach 'cleanup_repo'

  It 'reuses verification when all fingerprints match'
    When run bash -c '
      set -e
      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . ./bin/dev
      write_check_record
      ./.githooks/verify-checks
    '
    The status should be success
    The output should include '✅ Already verified — current content matches .last-checked.json'
  End

  It 'does not accept legacy timestamp markers as proof'
    When run bash -c '
      cd "$repo"
      touch .last-checked
      ./.githooks/verify-checks
    '
    The status should be failure
    The output should include '❌ You have un-verified changes. Run `dev check`'
    The output should include 'missing .last-checked.json'
  End

  It 'rejects verification when HEAD changes'
    When run bash -c '
      set -e
      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . ./bin/dev
      write_check_record
      printf "more\n" >> src/main.rs
      git add src/main.rs
      git commit --quiet -m change
      ./.githooks/verify-checks
    '
    The status should be failure
    The output should include 'HEAD changed'
  End

  It 'rejects verification when Cargo.lock changes after the marker was written'
    When run bash -c '
      set -e
      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . ./bin/dev
      write_check_record
      printf "changed\n" >> Cargo.lock
      ./.githooks/verify-checks
    '
    The status should be failure
    The output should include 'tracked files have unstaged changes'
  End

  It 'rejects verification when bin/dev changes after the marker was written'
    When run bash -c '
      set -e
      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . ./bin/dev
      write_check_record
      printf "# changed\n" >> bin/dev
      ./.githooks/verify-checks
    '
    The status should be failure
    The output should include 'tracked files have unstaged changes'
  End

  It 'rejects dirty tracked files explicitly'
    When run bash -c '
      set -e
      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . ./bin/dev
      write_check_record
      printf "dirty\n" >> src/main.rs
      ./.githooks/verify-checks
    '
    The status should be failure
    The output should include 'tracked files have unstaged changes'
  End

  It 'rejects staged tracked files explicitly'
    When run bash -c '
      set -e
      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . ./bin/dev
      write_check_record
      printf "staged\n" >> src/main.rs
      git add src/main.rs
      ./.githooks/verify-checks
    '
    The status should be failure
    The output should include 'index has staged changes'
  End

  It 'rejects untracked files explicitly'
    When run bash -c '
      set -e
      cd "$repo"
      YX_DEV_SOURCE_ONLY=1 . ./bin/dev
      write_check_record
      printf "local\n" > src/local_fixture.rs
      ./.githooks/verify-checks
    '
    The status should be failure
    The output should include 'untracked files are present'
  End
End
