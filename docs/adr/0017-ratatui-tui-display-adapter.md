# 17. Ratatui TuiDisplay adapter for rich terminal output

Date: 2026-03-02

## Status

Accepted

## Context

The CLI renders all output through the `DisplayPort` trait, implemented
by `ConsoleDisplay` which writes styled text using hand-rolled ANSI
escape codes to a `Box<dyn Write + Send>`. This works well but limits
visual richness — complex layouts like the `yx show` header box require
manual Unicode box-drawing and careful ANSI code management.

We wanted to add richer terminal UI using ratatui (v0.29, 18.7k GitHub
stars, used by Netflix/OpenAI/AWS). The key question was how to
integrate it without disrupting the existing architecture or test
infrastructure.

A spike investigated bubbletea-rs (v0.0.9, too immature) and ratatui,
and confirmed:

- `CrosstermBackend::new(stdout())` with `Viewport::Inline(height)`
  renders inline and exits — no alternate screen, no raw mode needed.
- `Viewport::Inline` requires a real TTY (fails with `Vec<u8>`).
- ratatui's `TestBackend` captures a `Buffer` of styled cells, ideal
  for unit testing without a terminal.
- Rendering widgets directly into a `Buffer` works for testing without
  even creating a `Terminal`.

## Decision

Introduce a second `DisplayPort` implementation alongside
`ConsoleDisplay`:

- **`ConsoleDisplay`** — unchanged. Plain text via `Write` trait, for
  piped output, CI, `NO_COLOR`, and non-TTY contexts.
- **`TuiDisplay`** — new. Uses ratatui widgets rendered through
  `CrosstermBackend` with `Viewport::Inline` for TTY output.

Routing at startup in `main.rs`: if stdout is a TTY and `NO_COLOR` is
not set, use `TuiDisplay`; otherwise use `ConsoleDisplay`.

Methods are converted incrementally. `TuiDisplay` holds a
`ConsoleDisplay` fallback — unconverted methods delegate to it,
preserving colours and behaviour until each method gets its own ratatui
rendering.

### Production rendering pattern

```rust
let backend = CrosstermBackend::new(io::stdout());
let mut terminal = Terminal::with_options(
    backend,
    TerminalOptions {
        viewport: Viewport::Inline(height),
    },
)?;
terminal.draw(|frame| { /* render widgets */ })?;
terminal.show_cursor()?;
println!(); // newline so shell prompt lands below
```

No raw mode. No alternate screen. Output persists in terminal history.

### Testing pattern

The draw logic is in `draw_header_box<B: Backend>`, generic over the
backend. Tests create a `TestBackend` with `Viewport::Fixed`:

```rust
let backend = TestBackend::new(width, height);
let mut terminal = Terminal::with_options(
    backend,
    TerminalOptions {
        viewport: Viewport::Fixed(Rect::new(0, 0, width, height)),
    },
)?;
display.draw_header_box(&mut terminal, ...);

let output = buffer_to_string(terminal.backend().buffer());
assert!(output.contains("my yak"));

// Cell-level style assertions:
let cell = &terminal.backend().buffer()[(x, y)];
assert!(cell.modifier.contains(Modifier::BOLD));
```

This exercises the full `terminal.draw()` path — height calculation,
frame rendering, cursor management — without needing a real TTY.

### What NOT to do

Do not hand-roll ANSI escape codes from ratatui `Buffer` cells. Use
`CrosstermBackend` for production output — it handles all styles,
colours, and terminal protocols correctly.

## Consequences

- Rich terminal UI can be added incrementally, one `DisplayPort` method
  at a time, without breaking existing behaviour.
- The `ConsoleDisplay` fallback means partially-converted state always
  works — no big-bang migration needed.
- Two display adapters to maintain, but the fallback delegation keeps
  the unconverted surface minimal.
- New dependency: `ratatui` (+ transitive `crossterm`). Note:
  ratatui 0.29 pulls in `lru` which has RUSTSEC-2026-0002 (unsound
  `IterMut`). This is a warning-level advisory, not a vulnerability.
  Monitor for ratatui upgrades that resolve it.
- Unit tests exercise the full draw path via `TestBackend`. The only
  untested production code is the `CrosstermBackend` +
  `Viewport::Inline` construction (needs a real TTY). Consider
  `ratatui-testlib` (PTY-based) for Cucumber integration tests later.
