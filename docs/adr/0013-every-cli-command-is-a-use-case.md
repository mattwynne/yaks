# 13. Every CLI command is a UseCase

Date: 2026-02-28

## Status

proposed

## Context

ADR 0008 established that `main.rs` should only parse arguments,
wire adapters, and route commands to use cases. In practice,
several commands had drifted from this principle — calling
methods directly on `Application` or doing domain logic inline
in `main.rs`.

An audit of all CLI commands found several cases where
operations bypassed the `UseCase` pattern:
- `compact` was a method on `Application`, not a `UseCase`
- `context --edit` orchestrates editor interaction in `main.rs`
- `field --edit` does the same
- `reset` contains extensive git and domain logic in `main.rs`
- `completions` reads directly from the store adapter

## Decision

Every CLI command MUST route through a `UseCase` struct via
`app.handle()`. No exceptions.

A `UseCase` struct:
1. **Captures the command's intent** — its fields are the
   parameters the user provided (name, flags, content)
2. **Implements `UseCase` trait** — a single `execute` method
   that receives `&mut Application`
3. **Owns all orchestration** — editor launching, multi-step
   workflows, confirmation prompts all belong in the use case,
   not in `main.rs`

`main.rs` becomes a pure routing table:

```rust
match command {
    Commands::Add { name, .. } => app.handle(AddYak::new(&name)),
    Commands::Compact => app.handle(CompactEvents::new()),
    Commands::Reset { .. } => app.handle(ResetYaks::new(mode)),
    // ...every command follows this pattern
}
```

### What belongs where

| Layer | Responsibility |
|-------|---------------|
| `main.rs` | Parse CLI args, wire adapters, route to use case |
| `UseCase` | Orchestrate the operation, call domain + ports |
| `Application` | Hold adapter references, provide helpers like `with_yak_map()` |
| Domain | Business rules, event generation |
| Adapters | Infrastructure (git, filesystem, editor) |

### Rules for main.rs

`main.rs` MUST NOT:
- Call any method on `Application` other than `handle()`
- Access adapter references (store, event_store, etc.) directly
- Contain domain logic, orchestration, or control flow beyond
  argument parsing and command routing
- Import domain types other than use case structs

### Rules for Application

`Application` MUST NOT grow new public methods without explicit
approval from the project owner. It exists to hold adapter
references and provide a small set of shared helpers (like
`with_yak_map()`) that multiple use cases need.

Any operation that corresponds to a CLI command MUST be a
`UseCase`, not a method on `Application`. When tempted to add
a method to `Application`, create a `UseCase` instead.

### Structural enforcement

Once all commands route through `handle()`, the compiler can
enforce the rule. Extract routing into a function that only
sees a `CommandHandler` trait:

```rust
// application/mod.rs
pub trait CommandHandler {
    fn handle(&mut self, use_case: impl UseCase) -> Result<()>;
}

impl CommandHandler for Application<'_> {
    fn handle(&mut self, use_case: impl UseCase) -> Result<()> {
        use_case.execute(self)
    }
}
```

```rust
// main.rs
fn main() {
    // Wiring: main sees Application to construct it
    let mut app = Application::new(...);
    // Routing: only sees CommandHandler
    route_command(command, &mut app)?;
}

fn route_command(
    cmd: Commands,
    handler: &mut impl CommandHandler,
) -> Result<()> {
    match cmd {
        Commands::Add { name, .. } =>
            handler.handle(AddYak::new(&name)),
        Commands::Compact =>
            handler.handle(CompactEvents::new()),
        // ...every command follows the same shape
    }
}
```

`route_command` physically cannot access adapters or call
anything except `handler.handle()`. The compiler enforces the
architectural rule — no discipline required.

This enforcement is blocked until the remaining commands are
extracted into use cases (context --edit, field --edit, reset,
completions).

## Consequences

### What becomes easier

- **Testing**: Any CLI operation can be tested with the standard
  setup: construct an `Application` with in-memory adapters,
  call `app.handle(MyUseCase::new(...))`, assert on output.
  No special test helpers or direct method calls needed.
- **Discoverability**: `src/application/mod.rs` is a catalogue
  of everything the system can do. Each use case is a single
  file with a clear name.
- **Composition in tests**: Tests can chain use cases naturally:
  `app.handle(AddYak::new("x").with_context(Some("notes")))`,
  then `app.handle(CompactEvents::new())`, then
  `app.handle(ShowLog::new())`.

### What becomes harder

- **Small commands feel over-structured**: A command like
  `completions` that just reads and filters a list now needs
  its own file and struct. This is intentional — consistency
  is worth more than brevity.
- **Editor interaction**: Use cases that launch `$EDITOR` need
  an `InputPort` method for interactive editing. The use case
  calls the port; `main.rs` doesn't orchestrate the editor.
