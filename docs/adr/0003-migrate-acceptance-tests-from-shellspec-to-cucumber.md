# 3. Migrate acceptance tests from ShellSpec to Cucumber

Date: 2026-02-13

## Status

accepted

## Context

ADR 0001 migrated the implementation from Bash to Rust but kept
ShellSpec as the test framework. This was pragmatic: it proved
the Rust implementation matched the original behaviour. But
ShellSpec tests are black-box only — they spawn the compiled
binary, interact via stdin/stdout, and assert on output. They
can't test the Application layer or domain logic in isolation.

ADR 0002 introduced CQRS with Event Sourcing and a hexagonal
architecture with in-memory adapters. This created a testability
surface that ShellSpec cannot reach: the `Application` struct
can be driven directly without I/O.

ShellSpec has further drawbacks:

- **Speed**: Each test spawns a process and writes to the
  filesystem. This adds up across 100+ tests.
- **Wrong language**: Tests are written in bash alongside a Rust
  codebase — two languages, two mental models, two toolchains.
- **Opaque specifications**: Test logic is embedded in shell
  scripting idioms. The intent is hard to read and harder to
  discuss.
- **Poor collaboration medium**: Requirements live in
  conversation or yak context fields, then get translated into
  shell scripts. There's no shared artefact where humans and
  agents can negotiate what "done" looks like.

## Decision

Migrate acceptance tests from ShellSpec to Cucumber, using the
Rust `cucumber` crate with a dual-mode execution pattern.

### Dual-mode testing

Every `.feature` file runs in two modes via the same step
definitions:

- **Full-stack mode**: spawns the `yx` binary, exercises the
  real CLI end-to-end (like ShellSpec does today).
- **In-process mode**: drives `Application` directly with
  in-memory adapters — no I/O, no process spawning.

Both modes share a `TestWorld` trait so identical Gherkin
scenarios validate behaviour at both levels. This was proven
with `list.feature` (16 scenarios, 79 steps, both modes green).

### Gherkin as the requirements medium

Feature files are the primary artefact for discussing and
agreeing on behaviour between humans and agents. They use
Cucumber's `Rule:` keyword to organise examples by business
rule, mapping directly to the Example Mapping technique (see
the `/example-mapping` skill). A feature file can be drafted,
discussed, and refined before any implementation begins.

### Incremental migration

Tests are converted one command at a time. ShellSpec tests for
a command are deleted once the Cucumber equivalent passes in
both modes.

## Consequences

### What becomes easier

**Fast feedback.** In-process mode runs the full list suite in
~0.00s vs ~0.43s full-stack. As coverage grows, this compounds.

**Architectural validation.** In-process tests exercise the
hexagonal architecture and CQRS/ES patterns directly. If an
in-memory adapter drifts from the real adapter's contract, the
full-stack mode catches it. The two modes keep each other
honest.

**Requirements collaboration.** Gherkin gives humans and agents
a shared language for negotiating behaviour before writing code.
Feature files replace the current pattern of describing
requirements in conversation then translating to shell scripts.

**Readable specifications.** Feature files organised by `Rule:`
blocks read as documentation, not test code.

**Single language stack.** Step definitions are Rust. No more
maintaining bash test code alongside a Rust codebase.

### What becomes harder

**Dual step definitions.** Each step must be implemented for
both worlds. The `TestWorld` trait abstracts this, but new
commands still require wiring up both execution paths.

**In-process fidelity.** In-process mode bypasses CLI parsing
(clap), ANSI formatting, and filesystem behaviour. Bugs in
these layers are only caught by full-stack mode.

**Migration effort.** ~30 ShellSpec files covering 11+ commands
need converting, each requiring Gherkin scenarios, dual step
definitions, and parity verification.

### Open questions

- **ShellSpec end state**: Remove entirely, or keep a thin
  smoke suite? Full-stack Cucumber mode may make a separate
  ShellSpec suite redundant, but this should be evaluated once
  migration is further along.
- **Commands that challenge dual-mode**: `sync` involves git
  operations across repositories, `completions` generates
  shell-specific output. Some tests may only run in full-stack
  mode, or may need new adapters to work in-process.
