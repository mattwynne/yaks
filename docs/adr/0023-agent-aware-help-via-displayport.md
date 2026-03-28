# 23. Agent-aware help via DisplayPort

Date: 2026-03-28

## Status

proposed

## Context

`yx --help` shows the same examples to every caller. Humans and AI
agents need different examples — humans want a getting-started flow,
agents want flags like `--format json` and stdin patterns like
heredocs.

Clap currently owns help rendering directly in `main.rs` via the
`#[command(after_help)]` attribute. This means the help string is
static, and clap — an infrastructure detail — is exposed at the
routing layer rather than contained behind an adapter.

In a hexagonal architecture, the CLI is one possible user interface.
A web app, REST API, or mobile client would each present help
differently. Clap should sit inside a CLI adapter, not in the
application's routing layer.

`LocalWorkspacePort` (ADR 0021) already provides workspace-level
environment detection and is the natural home for agent session
detection.

## Decision

**Agent detection:** Add `is_agent_session()` to `LocalWorkspacePort`.
The real adapter checks `CLAUDECODE=1`, extensible to other agent
environment variables. This replaces the `in_claude_code_session()`
free function in `main.rs`.

**Help rendering:** Add `show_help(is_agent: bool)` to `DisplayPort`.
The display adapter uses clap internally to build and render help
with the appropriate examples. Clap becomes an infrastructure detail
inside the adapter.

**Help as a use case:** A `ShowHelp` use case calls
`workspace.is_agent_session()` and passes the result to
`display.show_help(is_agent)`. Both the no-args path and `--help`
route to this use case.

## Consequences

### Easier

- Help examples are tailored to the caller — agents see patterns
  they actually use (heredocs, `--format json`, piped stdin),
  humans see a friendly getting-started flow.
- Clap is contained inside the display adapter, not exposed at the
  routing layer. If we ever swap CLI frameworks or add a non-CLI
  interface, help rendering adapts naturally.
- Agent detection is behind a port and testable with in-memory
  adapters. Cucumber scenarios can verify both example sets without
  environment variable manipulation.

### Harder

- `--help` must be intercepted before clap's built-in handler
  calls `std::process::exit`. This adds a small amount of
  pre-parse detection in `main.rs`.
- Display adapters gain a new method, and the clap dependency moves
  into them. Adapters that don't need clap-formatted help (e.g.
  `JsonDisplay`) still need an implementation.
