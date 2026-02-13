# Unify done and state commands Implementation Plan

> **For Claude:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `yx done` and `yx start` sugar commands that delegate to `yx state`, adding fuzzy matching and `--recursive` to `yx state`.

**Architecture:** Move fuzzy matching and recursive logic from `DoneYak` into `SetState`. Rewrite `DoneYak` and add `StartYak` as thin wrappers. Update completions and CLI wiring.

**Tech Stack:** Rust, clap, ShellSpec

**Spec:** `docs/superpowers/specs/2026-02-12-unify-done-and-state-commands.md`

---

## Chunk 1: Add fuzzy matching and --recursive to SetState

### Task 1: Add fuzzy matching to `yx state`

Currently `yx state` uses exact names only. `yx done` has fuzzy matching (leaf-node substring). Move fuzzy matching into `SetState`.

**Files:**
- Modify: `src/application/set_state.rs`
- Test: `spec/features/state.sh`

- [ ] **Step 1: Write failing test for fuzzy match on state**

Add to `spec/features/state.sh` before the final `End`:

```bash
  It 'resolves yak name with fuzzy matching'
    When run sh -c "
      yx add 'Fix the bug'
      yx state bug wip
      yx list --format markdown
    "
    The output should include "- [wip] Fix the bug"
  End

  It 'shows error when fuzzy match is ambiguous'
    When run sh -c "
      yx add 'Fix the bug'
      yx add 'Report the bug'
      yx state bug wip
    "
    The error should include "yak name 'bug' is ambiguous"
    The status should be failure
  End
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `shellspec --pattern 'spec/features/state.sh'`
Expected: 2 failures (fuzzy match not implemented in SetState)

- [ ] **Step 3: Implement fuzzy matching in SetState**

Replace `src/application/set_state.rs` with:

```rust
// Use case: Set a yak's state

use anyhow::Result;

use super::{Application, UseCase};

pub struct SetState {
    name: String,
    state: String,
    recursive: bool,
}

impl SetState {
    pub fn new(name: &str, state: &str) -> Self {
        Self {
            name: name.to_string(),
            state: state.to_string(),
            recursive: false,
        }
    }

    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    fn resolve_name(&self, app: &Application) -> Result<String> {
        let all_yaks = app.store.list_yaks()?;
        let name = &self.name;

        if app.store.yak_exists(name) {
            return Ok(name.clone());
        }

        let matches: Vec<String> = all_yaks
            .iter()
            .filter(|yak| {
                let leaf = yak.name.rsplit('/').next().unwrap_or(&yak.name);
                leaf.contains(name.as_str())
            })
            .map(|yak| yak.name.clone())
            .collect();

        match matches.len() {
            0 => anyhow::bail!("yak '{}' not found", name),
            1 => Ok(matches[0].clone()),
            _ => anyhow::bail!("yak name '{}' is ambiguous", name),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let resolved_name = self.resolve_name(app)?;

        app.with_yak_map(move |yak_map| {
            yak_map.update_state(resolved_name, self.state.clone())
        })
    }
}

impl UseCase for SetState {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

Note: `recursive` field is added but not used yet - that's Task 2.

- [ ] **Step 4: Run tests to verify they pass**

Run: `shellspec --pattern 'spec/features/state.sh'`
Expected: all 9 examples pass (7 existing + 2 new)

- [ ] **Step 5: Commit**

```bash
git mit me && git add src/application/set_state.rs spec/features/state.sh && git commit -m "Add fuzzy matching to yx state"
```

### Task 2: Add --recursive to `yx state`

**Files:**
- Modify: `src/application/set_state.rs`
- Modify: `src/main.rs` (add --recursive arg to State command)
- Test: `spec/features/state.sh`

- [ ] **Step 1: Write failing test for recursive state**

Add to `spec/features/state.sh` before the final `End`:

```bash
  It 'sets state recursively on parent and all descendants'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child1'
      yx add 'parent/child2'
      yx add 'parent/child1/grandchild'
      yx state --recursive 'parent' done
      yx list --format markdown
    "
    The output should include $'\e[90m- [done] parent\e[0m'
    The output should include $'\e[90m  - [done] child1\e[0m'
    The output should include $'\e[90m  - [done] child2\e[0m'
    The output should include $'\e[90m    - [done] grandchild\e[0m'
  End
```

- [ ] **Step 2: Run tests to verify it fails**

Run: `shellspec --pattern 'spec/features/state.sh'`
Expected: 1 failure (--recursive not recognized)

- [ ] **Step 3: Add --recursive flag to CLI and implement in SetState**

In `src/main.rs`, update the `State` variant:

```rust
    /// Set the state of a yak
    State {
        /// The yak name (space-separated words)
        name: Vec<String>,
        /// The state to set (e.g., "todo", "wip", "done")
        state: String,
        /// Apply state change recursively to all descendants
        #[arg(long)]
        recursive: bool,
    },
```

Update the `State` match arm in `main()`:

```rust
        Commands::State { name, state, recursive } => {
            let name_str = name.join(" ");
            app.handle(SetState::new(&name_str, &state).with_recursive(recursive))
        }
```

In `src/application/set_state.rs`, update the `execute` method:

```rust
    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let resolved_name = self.resolve_name(app)?;

        let names_to_update = if self.recursive {
            let all_yaks = app.store.list_yaks()?;
            let mut names: Vec<String> = all_yaks
                .iter()
                .filter(|yak| {
                    yak.name == resolved_name
                        || yak.name.starts_with(&format!("{resolved_name}/"))
                })
                .map(|yak| yak.name.clone())
                .collect();
            // Sort by depth descending (leaves first) so children are
            // marked done before parents, passing hierarchy validation
            names.sort_by(|a, b| {
                let depth_a = a.matches('/').count();
                let depth_b = b.matches('/').count();
                depth_b.cmp(&depth_a)
            });
            names
        } else {
            vec![resolved_name]
        };

        let state = self.state.clone();
        app.with_yak_map(move |yak_map| {
            for name in names_to_update {
                yak_map.update_state(name, state.clone())?;
            }
            Ok(())
        })
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `shellspec --pattern 'spec/features/state.sh'`
Expected: all 10 examples pass

- [ ] **Step 5: Commit**

```bash
git mit me && git add src/application/set_state.rs src/main.rs spec/features/state.sh && git commit -m "Add --recursive flag to yx state"
```

## Chunk 2: Rewrite DoneYak and add StartYak as sugar commands

### Task 3: Rewrite DoneYak to delegate to SetState

**Files:**
- Modify: `src/application/done_yak.rs`
- Modify: `src/main.rs` (remove --undo from Done command)
- Modify: `spec/features/done.sh` (remove undo test, fix WIP propagation test)
- Modify: `src/application/completions.rs` (remove --undo from completions)

- [ ] **Step 1: Update done.sh tests**

Remove the undo test (lines 52-60) and update the spec to reflect the new behaviour. The full updated file:

```bash
# shellcheck shell=bash
# shellcheck disable=SC1010
Describe 'yx done'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'marks a yak as done'
    When run sh -c "
      yx add 'Fix the bug'
      yx done 'Fix the bug'
      yx list --format markdown
    "
    The output should include $'\e[90m- [done] Fix the bug\e[0m'
  End

  It 'shows error when marking non-existent yak as done'
    When run yx done "Nonexistent yak"
    The error should include "Error: yak 'Nonexistent yak' not found"
    The status should be failure
  End

  It 'displays mix of done and not-done yaks'
    When run sh -c "
      yx add 'Fix the bug'
      yx add 'Write the docs'
      yx add 'Add tests'
      yx done 'Write the docs'
      yx list --format markdown
    "
    The output should include "- [todo] Fix the bug"
    The output should include $'\e[90m- [done] Write the docs\e[0m'
    The output should include "- [todo] Add tests"
  End

  It 'handles yak names starting with x'
    When run sh -c "
      yx add 'x marks the spot'
      yx list --format markdown
    "
    The output should include "- [todo] x marks the spot"
  End

  It 'marks yak starting with x as done correctly'
    When run sh -c "
      yx add 'x marks the spot'
      yx done 'x marks the spot'
      yx list --format markdown
    "
    The output should include $'\e[90m- [done] x marks the spot\e[0m'
  End

  It 'marks a nested yak as done'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx done 'parent/child'
      yx list --format markdown
    "
    The line 1 should equal "- [wip] parent"
    The line 2 should equal $'\e[90m  - [done] child\e[0m'
  End

  It 'errors when marking a parent yak as done with incomplete children'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx done 'parent'
    "
    The error should include "Error: cannot mark 'parent' as done - it has incomplete children"
    The status should be failure
  End

  It 'marks parent and all children as done with --recursive flag'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child1'
      yx add 'parent/child2'
      yx add 'parent/child1/grandchild'
      yx done --recursive 'parent'
      yx list --format markdown
    "
    The output should include $'\e[90m- [done] parent\e[0m'
    The output should include $'\e[90m  - [done] child1\e[0m'
    The output should include $'\e[90m  - [done] child2\e[0m'
    The output should include $'\e[90m    - [done] grandchild\e[0m'
  End
End
```

- [ ] **Step 2: Rewrite DoneYak to delegate to SetState**

Replace `src/application/done_yak.rs`:

```rust
// Use case: Mark a yak as done (sugar for SetState with state="done")

use anyhow::Result;

use super::{Application, SetState, UseCase};

pub struct DoneYak {
    name: String,
    recursive: bool,
}

impl DoneYak {
    pub fn new(name: &str, recursive: bool) -> Self {
        Self {
            name: name.to_string(),
            recursive,
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        SetState::new(&self.name, "done")
            .with_recursive(self.recursive)
            .execute(app)
    }
}

impl UseCase for DoneYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

- [ ] **Step 3: Update CLI wiring - remove --undo from Done command**

In `src/main.rs`, update the Done variant:

```rust
    /// Mark yak as done
    #[command(alias = "finish")]
    Done {
        /// The yak name (space-separated words)
        name: Vec<String>,
        /// Mark yak and all children as done recursively
        #[arg(long)]
        recursive: bool,
    },
```

Update the Done match arm:

```rust
        Commands::Done {
            name,
            recursive,
        } => {
            let name_str = name.join(" ");
            app.handle(DoneYak::new(&name_str, recursive))
        }
```

- [ ] **Step 4: Update completions - remove --undo**

In `src/application/completions.rs`, update the `command_flags` closure:

```rust
            "done" | "finish" => vec!["--recursive"],
```

Remove the `done_undo_shows_only_done_yaks` test (lines 183-188) and the `--undo` references from `offers_flags_for_done` (lines 191-195) and `offers_flags_and_yaks_together` (lines 203-210).

Updated `offers_flags_for_done`:
```rust
    #[test]
    fn offers_flags_for_done() {
        let result = complete_with_state(&["yx", "done", "--"], &[]);
        assert!(result.contains(&"--recursive".to_string()));
        assert!(!result.contains(&"--undo".to_string()));
    }
```

Updated `offers_flags_and_yaks_together`:
```rust
    #[test]
    fn offers_flags_and_yaks_together() {
        let yaks = &[("my-yak", false)];
        let result = complete_with_state(&["yx", "done", ""], yaks);
        assert!(result.contains(&"my-yak".to_string()));
        assert!(result.contains(&"--recursive".to_string()));
    }
```

In the `complete_with_state` function, remove the `--undo` completion filtering logic (lines 73-84). Replace smart filtering for done/finish:

```rust
            let filtered_yaks: Vec<_> = if subcommand == "done" || subcommand == "finish" {
                // Show only incomplete yaks for done operations
                yaks.iter().filter(|(_, is_done)| !*is_done).collect()
            } else if subcommand == "start" || subcommand == "wip" {
                // Show only non-wip yaks for start operations
                yaks.iter().collect()
            } else {
                // For other commands, show all yaks
                yaks.iter().collect()
            };
```

- [ ] **Step 5: Run all done and state tests**

Run: `shellspec --pattern 'spec/features/done.sh' --pattern 'spec/features/state.sh'`
Expected: all tests pass (the "marks a nested yak as done" test should now pass because DoneYak delegates to SetState which has WIP propagation)

- [ ] **Step 6: Run cargo tests**

Run: `cargo test --lib` in the worktree
Expected: all pass (completions_match_cli_commands will fail - fixed in Task 4)

- [ ] **Step 7: Commit**

```bash
git mit me && git add src/application/done_yak.rs src/main.rs spec/features/done.sh src/application/completions.rs && git commit -m "Rewrite DoneYak as sugar for SetState

Remove --undo flag. Fixes WIP propagation
when marking nested yak done via yx done."
```

### Task 4: Add StartYak use case and CLI command

**Files:**
- Create: `src/application/start_yak.rs`
- Modify: `src/application/mod.rs` (add module + export)
- Modify: `src/main.rs` (add Start command, update import)
- Modify: `src/application/completions.rs` (add start/wip to COMMANDS and completions)
- Create: `spec/features/start.sh`

- [ ] **Step 1: Write tests for yx start**

Create `spec/features/start.sh`:

```bash
# shellcheck shell=bash
# shellcheck disable=SC1010
Describe 'yx start'
  BeforeEach 'setup_isolated_repo'
  AfterEach 'teardown_isolated_repo'

  It 'sets a yak to wip state'
    When run sh -c "
      yx add 'Fix the bug'
      yx start 'Fix the bug'
      yx list --format markdown
    "
    The output should include "- [wip] Fix the bug"
  End

  It 'shows error when starting non-existent yak'
    When run yx start "Nonexistent yak"
    The error should include "Error: yak 'Nonexistent yak' not found"
    The status should be failure
  End

  It 'resolves yak name with fuzzy matching'
    When run sh -c "
      yx add 'Fix the bug'
      yx start bug
      yx list --format markdown
    "
    The output should include "- [wip] Fix the bug"
  End

  It 'propagates wip to parent'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child'
      yx start 'parent/child'
      yx list --format markdown
    "
    The line 1 should equal "- [wip] parent"
    The line 2 should equal "  - [wip] child"
  End

  It 'sets state recursively on parent and all descendants'
    When run sh -c "
      yx add 'parent'
      yx add 'parent/child1'
      yx add 'parent/child2'
      yx start --recursive 'parent'
      yx list --format markdown
    "
    The output should include "- [wip] parent"
    The output should include "  - [wip] child1"
    The output should include "  - [wip] child2"
  End

  It 'works with wip alias'
    When run sh -c "
      yx add 'Fix the bug'
      yx wip 'Fix the bug'
      yx list --format markdown
    "
    The output should include "- [wip] Fix the bug"
  End
End
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `shellspec --pattern 'spec/features/start.sh'`
Expected: all fail (command doesn't exist)

- [ ] **Step 3: Create StartYak use case**

Create `src/application/start_yak.rs`:

```rust
// Use case: Start a yak (sugar for SetState with state="wip")

use anyhow::Result;

use super::{Application, SetState, UseCase};

pub struct StartYak {
    name: String,
    recursive: bool,
}

impl StartYak {
    pub fn new(name: &str, recursive: bool) -> Self {
        Self {
            name: name.to_string(),
            recursive,
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        SetState::new(&self.name, "wip")
            .with_recursive(self.recursive)
            .execute(app)
    }
}

impl UseCase for StartYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
```

- [ ] **Step 4: Register module and export**

In `src/application/mod.rs`, add:
- After `mod set_state;`: `mod start_yak;`
- After `pub use set_state::SetState;`: `pub use start_yak::StartYak;`

- [ ] **Step 5: Wire up CLI**

In `src/main.rs`, add to imports:

```rust
use yx::application::{
    complete_with_state, AddYak, Application, DoneYak, EditContext, ListYaks, MoveYak, PruneYaks,
    RemoveYak, SetState, ShowContext, ShowField, ShowLog, StartYak, SyncYaks, WriteField,
};
```

Add the Start variant to the Commands enum (after Done):

```rust
    /// Start working on a yak (set state to wip)
    #[command(alias = "wip")]
    Start {
        /// The yak name (space-separated words)
        name: Vec<String>,
        /// Start yak and all children recursively
        #[arg(long)]
        recursive: bool,
    },
```

Add the match arm (after the Done arm):

```rust
        Commands::Start {
            name,
            recursive,
        } => {
            let name_str = name.join(" ");
            app.handle(StartYak::new(&name_str, recursive))
        }
```

- [ ] **Step 6: Update completions**

In `src/application/completions.rs`:

Add to `COMMANDS` array (after `"finish"`):
```rust
    "start",
    "wip",
```

Add to `commands_with_yak_args`:
```rust
    let commands_with_yak_args = vec![
        "done", "finish", "start", "wip", "remove", "rm", "move", "mv", "context", "state", "field",
    ];
```

Add flags for start/wip in `command_flags`:
```rust
            "start" | "wip" => vec!["--recursive"],
```

Also add `--recursive` to `"state"`:
```rust
            "state" => vec!["--recursive"],
```

- [ ] **Step 7: Run all tests**

Run: `shellspec --pattern 'spec/features/start.sh' --pattern 'spec/features/done.sh' --pattern 'spec/features/state.sh'`
Expected: all pass

Run: `cargo test --lib`
Expected: all pass (including completions_match_cli_commands)

- [ ] **Step 8: Commit**

```bash
git mit me && git add src/application/start_yak.rs src/application/mod.rs src/main.rs src/application/completions.rs spec/features/start.sh && git commit -m "Add yx start/wip command

Sugar for yx state <name> wip, with fuzzy
matching and --recursive support."
```

## Chunk 3: Final cleanup and verification

### Task 5: Run full checks and clean up

- [ ] **Step 1: Run dev check**

Run: `dev check`
Expected: all checks pass (tests + lint + audit)

- [ ] **Step 2: Fix any clippy/rustfmt issues if needed**

If `dev check` reports formatting or lint issues, fix them and commit.

- [ ] **Step 3: Verify the pre-existing done test is now fixed**

Run: `shellspec --pattern 'spec/features/done.sh'`

Confirm "marks a nested yak as done" passes (it was failing before because DoneYak didn't propagate WIP to ancestors - now it delegates to SetState which does).
