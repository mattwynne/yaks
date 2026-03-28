# Agent-Aware Help Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show different help examples to humans vs AI agents, with help rendering behind `DisplayPort` and agent detection on `LocalWorkspacePort`.

**Architecture:** `is_agent_session()` on `LocalWorkspacePort` detects agent callers. `show_help(is_agent: bool)` on `DisplayPort` renders help with clap internally. `main.rs` intercepts `--help`/no-args before `Cli::parse()`, constructs only the two ports needed, and calls `display.show_help(workspace.is_agent_session())` directly. No `Application` or full adapter construction required — help must work outside a git repo.

**Future work:** Moving `Cli` into the library crate and making help a proper use case via `app.handle(ShowHelp::new())` is tracked in the "move Cli into library crate" yak.

**Tech Stack:** Rust, clap (derive), Cucumber (acceptance tests)

**Spec:** Run `yx context --show "machine-friendly help output when CLAUDECODE is set"` to read the full spec.

**ADR:** `docs/adr/0023-agent-aware-help-via-displayport.md`

---

### Task 1: Add `is_agent_session()` to `LocalWorkspacePort`

**Files:**
- Modify: `src/domain/ports/local_workspace.rs`
- Modify: `src/adapters/local_workspace.rs`

Grep for `impl LocalWorkspacePort` across the entire codebase — every implementation needs the new method. Known sites: `LocalWorkspace` in `src/adapters/local_workspace.rs`, `TestWorkspace` in `tests/features/in_process_world.rs`, `TestWorkspace` in `src/application/show_log.rs`, `TestWorkspace` in `src/application/app.rs`, `TestWorkspace` in `src/application/ensure_gitignore.rs`, and `NullWorkspace` (grep for it).

- [ ] **Step 1: Add method to the port trait**

In `src/domain/ports/local_workspace.rs`, add to `LocalWorkspacePort`:

```rust
/// Check whether the current session is driven by an AI agent.
///
/// Used to tailor CLI output (e.g. help examples) for agents.
/// The real adapter checks environment variables like CLAUDECODE=1.
fn is_agent_session(&self) -> bool;
```

- [ ] **Step 2: Implement on `LocalWorkspace` adapter**

In `src/adapters/local_workspace.rs`, add:

```rust
fn is_agent_session(&self) -> bool {
    std::env::var("CLAUDECODE").as_deref() == Ok("1")
}
```

- [ ] **Step 3: Implement on every other `impl LocalWorkspacePort`**

For each one found by grep, add:

```rust
fn is_agent_session(&self) -> bool {
    false
}
```

- [ ] **Step 4: Run tests to verify compilation**

Run: `cargo test --lib --no-run`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```
Add is_agent_session to LocalWorkspacePort
```

---

### Task 2: Add `show_help(is_agent: bool)` to `DisplayPort`

**Files:**
- Modify: `src/domain/ports/user_display.rs`
- Modify: `src/adapters/user_display/mod.rs` (ConsoleDisplay)
- Modify: `src/adapters/tui_display/mod.rs` (TuiDisplay)
- Modify: `src/adapters/json_display.rs` (JsonDisplay)

Grep for `impl DisplayPort` to find all implementations. Note: `make_test_display()` returns a `ConsoleDisplay` — there is no separate test display adapter.

- [ ] **Step 1: Add method to the port trait**

In `src/domain/ports/user_display.rs`, add to `DisplayPort`:

```rust
/// Display CLI help, with examples tailored for the audience.
///
/// When `is_agent` is true, show examples optimised for AI agents
/// (e.g. --format json, heredoc stdin patterns).
fn show_help(&self, is_agent: bool);
```

- [ ] **Step 2: Implement on `ConsoleDisplay`**

In `src/adapters/user_display/mod.rs`. The adapter imports `Cli` from `main.rs` — but `Cli` is in the binary crate. Since `Cli` can't be imported from the library, the adapter needs to build a `clap::Command` directly. Use `clap::Command::new("yx")` with the same help template and styling, then inject the examples via `.after_help()`.

Alternatively, add a public function to `ConsoleDisplay` that accepts a `clap::Command` and renders it, so `main.rs` can pass `Cli::command()` in. This avoids duplicating the command definition.

**Recommended approach:** Add a `show_help_for_command(&self, cmd: clap::Command, is_agent: bool)` helper on the adapter, and have `show_help` call it. `main.rs` can call `show_help_for_command` directly with `Cli::command()`.

Actually, simpler: `show_help` takes `is_agent: bool` on the port. The adapter also needs the clap `Command` to render. Since `Cli` is in `main.rs`, the display adapter can't build the command. So `main.rs` must pre-build the help string and pass it to the display port.

**Revised port signature:**

```rust
/// Display pre-rendered help text to the user.
fn show_help(&self, rendered_help: &str);
```

And a standalone function in the adapter module handles example selection:

```rust
pub fn help_examples(is_agent: bool) -> &'static str { ... }
```

`main.rs` calls:
```rust
let examples = help_examples(workspace.is_agent_session());
let help = Cli::command().after_help(examples).render_help().to_string();
display.show_help(&help);
```

This keeps clap in `main.rs` (where `Cli` lives) and the display port just outputs text. The example strings live in the adapter module as an implementation detail.

**Use this approach.** Update the port signature:

```rust
/// Display help text to the user.
fn show_help(&self, help_text: &str);
```

- [ ] **Step 3: Implement on `ConsoleDisplay`**

```rust
fn show_help(&self, help_text: &str) {
    let mut out = self.output.lock().unwrap();
    write!(out, "{help_text}").ok();
}
```

- [ ] **Step 4: Implement on `TuiDisplay`**

Delegate to fallback:

```rust
fn show_help(&self, help_text: &str) {
    self.fallback.show_help(help_text);
}
```

- [ ] **Step 5: Implement on `JsonDisplay`**

```rust
fn show_help(&self, help_text: &str) {
    let mut writer = self.writer.lock().unwrap();
    write!(writer, "{help_text}").ok();
}
```

- [ ] **Step 6: Run tests to verify compilation**

Run: `cargo test --lib --no-run`
Expected: compiles successfully

- [ ] **Step 7: Commit**

```
Add show_help to DisplayPort
```

---

### Task 3: Add example strings module

**Files:**
- Create: `src/adapters/help_examples.rs`
- Modify: `src/adapters/mod.rs`

- [ ] **Step 1: Create the module**

Create `src/adapters/help_examples.rs` with two functions returning the example strings. Use ANSI escape codes for the yellow "Examples:" header (matching the existing style in `main.rs:44-51`).

See the spec for exact content: `yx context --show "machine-friendly help output when CLAUDECODE is set"`

```rust
pub fn for_humans() -> &'static str {
    "\
\x1b[1;33mExamples:\x1b[0m
  yx add \"fix the flaky test\"
  yx add \"upgrade auth library\" --under \"fix the flaky test\"
  yx list
  yx show \"fix the flaky test\"
  yx start \"fix the flaky test\"
  yx tag \"fix the flaky test\" bug
  yx field \"fix the flaky test\" priority <<< \"high\"
  yx list --tag bug
  yx done \"fix the flaky test\""
}

pub fn for_agents() -> &'static str {
    "\
\x1b[1;33mExamples:\x1b[0m
  yx add \"fix the flaky test\" --under \"parent yak\"
  yx show \"fix the flaky test\"
  yx context \"fix the flaky test\" <<< $(cat <<'EOF'
  The login test fails intermittently due to a race
  condition in the session cleanup code.
  EOF
  )
  yx field \"fix the flaky test\" plan <<< \"Step 1: ...\"
  yx tag \"fix the flaky test\" bug
  yx list --tag bug --format json
  yx done \"fix the flaky test\""
}
```

- [ ] **Step 2: Register in `src/adapters/mod.rs`**

Add `pub mod help_examples;`

- [ ] **Step 3: Add unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_examples_contain_start() {
        assert!(for_humans().contains("yx start"));
    }

    #[test]
    fn human_examples_do_not_contain_format_json() {
        assert!(!for_humans().contains("--format json"));
    }

    #[test]
    fn agent_examples_contain_format_json() {
        assert!(for_agents().contains("--format json"));
    }

    #[test]
    fn agent_examples_do_not_contain_start() {
        assert!(!for_agents().contains("yx start"));
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib help_examples`
Expected: all 4 pass

- [ ] **Step 5: Commit**

```
Add help example strings for humans and agents
```

---

### Task 4: Wire help rendering in `main.rs`

**Files:**
- Modify: `src/main.rs`

Help must work outside a git repo. The current no-args path (line 680) exits before adapter construction. We keep this early-exit pattern but construct only `DisplayPort` + `LocalWorkspacePort` for the help path.

Note: `is_help_invocation()` (line 641) is also used by `maybe_show_claude_plugin_hint()` (line 626). Don't change that function — introduce a separate `wants_help` check.

- [ ] **Step 1: Remove `after_help` from `Cli` struct**

At `src/main.rs:44-51`, remove the `#[command(after_help = "...")]` attribute.

- [ ] **Step 2: Add help detection and rendering**

Replace the no-args block (lines 680-685) with a broader help check that also catches `--help`/`-h`:

```rust
// Detect top-level help: no args, or just --help/-h
let wants_help = args.len() == 1
    || (args.len() == 2 && args.iter().any(|a| a == "--help" || a == "-h"));

if wants_help {
    // Construct only the ports we need — help works outside a git repo
    let is_tty = std::io::stdout().is_terminal();
    let no_color = std::env::var_os("NO_COLOR").is_some();
    let guarded_stdout = Box::new(BrokenPipeGuard::new(std::io::stdout()));

    let display: Box<dyn yx::domain::ports::DisplayPort> = if is_tty && !no_color {
        Box::new(TuiDisplay::with_writer(guarded_stdout))
    } else {
        use yx::adapters::user_display::ConsoleDisplayOptions;
        let width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);
        Box::new(ConsoleDisplay::new(
            guarded_stdout,
            ConsoleDisplayOptions { color: false, width },
        ))
    };

    let workspace = yx::adapters::local_workspace::LocalWorkspace::default_without_repo();
    let examples = if workspace.is_agent_session() {
        yx::adapters::help_examples::for_agents()
    } else {
        yx::adapters::help_examples::for_humans()
    };

    let help = Cli::command().after_help(examples).render_help().to_string();
    display.show_help(&help);

    let exit_code = if args.len() == 1 { 2 } else { 0 };
    std::process::exit(exit_code);
}
```

Note: `LocalWorkspace::default_without_repo()` is a new constructor that doesn't need a repo root — `is_agent_session()` only checks env vars. If `LocalWorkspace::new()` requires a `PathBuf`, add a lightweight constructor, or just call the env var check directly here since this is infrastructure wiring in `main.rs`.

If adding a constructor feels like overengineering, just inline it:

```rust
let is_agent = std::env::var("CLAUDECODE").as_deref() == Ok("1");
let examples = if is_agent {
    yx::adapters::help_examples::for_agents()
} else {
    yx::adapters::help_examples::for_humans()
};
```

The port exists for testability in use cases. In `main.rs` wiring, calling the adapter (or even the env var directly) is fine.

- [ ] **Step 3: Run `dev check`**

Run: `dev check`
Expected: all checks pass

- [ ] **Step 4: Manual smoke test**

```bash
yx --help                    # human examples, exit 0
yx                           # human examples, exit 2
CLAUDECODE=1 yx --help       # agent examples, exit 0
CLAUDECODE=1 yx              # agent examples, exit 2
yx add --help                # subcommand help still works (clap handles it)
cd /tmp && yx --help         # works outside git repo
```

- [ ] **Step 5: Commit**

```
Show agent-aware help examples
```

---

### Task 5: Write acceptance tests

**Files:**
- Create: `features/help.feature`
- Modify: step definitions as needed

Help rendering happens in `main.rs` before `Application` construction, so these tests run via `FullStackWorld` only (real binary). Check existing feature files and step definitions for conventions.

- [ ] **Step 1: Check existing step patterns**

Read existing `.feature` files and step definition files to understand the Gherkin style, step phrasing, and how `FullStackWorld` runs the binary and captures output. Look for patterns like "When I run", "Then the output should contain", and how environment variables are set.

- [ ] **Step 2: Write the feature file**

Create `features/help.feature`. Adapt step phrasing to match existing conventions:

```gherkin
Feature: Help output

  Scenario: Human sees human-friendly examples
    When I run yx with no arguments
    Then the output should contain "yx start"
    And the output should not contain "--format json"

  Scenario: Agent sees agent-friendly examples
    Given I am in an agent session
    When I run yx with no arguments
    Then the output should contain "--format json"
    And the output should not contain "yx start"
```

- [ ] **Step 3: Implement any missing step definitions**

Steps likely needed:
- "Given I am in an agent session" — sets `CLAUDECODE=1` in the child process environment for the next yx invocation
- "When I run yx with no arguments" — runs `yx` with no args (check if an existing step like "When I run `yx`" already covers this)

Check how `FullStackWorld` manages env vars for child processes and follow that pattern.

- [ ] **Step 4: Run `dev check`**

Run: `dev check`
Expected: all checks pass

- [ ] **Step 5: Commit**

```
Test agent-aware help examples
```

---

### Task 6: Clean up `in_claude_code_session()`

**Files:**
- Modify: `src/main.rs`

The free function `in_claude_code_session()` at `src/main.rs:645-647` is now redundant — `LocalWorkspacePort::is_agent_session()` is the canonical check. Remove it and inline the env var check in its one caller (`maybe_show_claude_plugin_hint`).

- [ ] **Step 1: Inline and remove**

Replace `in_claude_code_session()` call in `maybe_show_claude_plugin_hint` with:

```rust
if std::env::var("CLAUDECODE").as_deref() != Ok("1") {
    return;
}
```

Delete the `in_claude_code_session()` function.

- [ ] **Step 2: Run `dev check`**

Run: `dev check`
Expected: all checks pass

- [ ] **Step 3: Commit**

```
Inline agent check in plugin hint
```
