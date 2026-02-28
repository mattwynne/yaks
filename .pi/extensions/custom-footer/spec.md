# Custom Footer Extension Spec

## Existing Default Footer

The default pi footer shows two lines. Here's a real example:

```
~/git/mattwynne/yaks (main) • "architecture review"
↑10k ↓20k R2.3M W64k $2.099 32.4%/200k (auto)                                                                          claude-opus-4-6 • medium
```

### Line 1 (location bar)
- Current working directory path
- Git branch in parentheses
- Session name in quotes (if set), preceded by `•`

### Line 2 (stats bar)
- Left side: token stats — `↑input ↓output R(cache-read) W(cache-write) $cost percentage/context-window (auto)`
- Right side: model ID • thinking level

## Desired Changes

The session name should **stand out more**. Specifically:

1. **Keep session name on the location bar** — same position as the default (after git branch, with `•` separator).
2. **Bold and accent-colored** — use `theme.bold()` and `theme.fg("accent", ...)` so it visually pops against the dim surroundings.
3. **No quotes** — the default wraps the session name in `"quotes"`. Remove them.
4. **Hidden when no session name** — if no session name is set, the `•` and name don't appear.
5. **Everything else unchanged** — the location bar and stats bar should look exactly like the default footer. Since we can't reuse the default footer renderer, we need to reproduce its content faithfully.

## Context Window Coloring

The stats bar colorizes the context percentage based on usage:
- **> 90%** → `theme.fg("error", ...)` (red)
- **> 70%** → `theme.fg("warning", ...)` (yellow)
- **Otherwise** → plain/dim

This must be preserved in the custom footer.

## Default Footer Source Code

The full source of the default footer is at:
`/nix/store/mws8dqyglrbr9ljhxk0iabaim63c4gqs-pi-0.55.0/lib/node_modules/@mariozechner/pi-coding-agent/dist/modes/interactive/components/footer.js`

**Read this file** — it contains the exact logic for token formatting, context percentage coloring, path truncation, git branch display, model/thinking display, provider prefix logic, extension status lines, and more. Your implementation should reproduce all of this behavior faithfully, with the only change being the added session name line.

## Implementation Notes

- This is a pi extension at `.pi/extensions/custom-footer/index.ts`
- Use `ctx.ui.setFooter()` to replace the footer entirely
- The footer factory receives `(tui, theme, footerData)` and returns `{ render(width): string[], invalidate(), dispose? }`
- `footerData.getGitBranch()` — returns current git branch or null
- `footerData.getExtensionStatuses()` — returns `ReadonlyMap<string, string>` of status texts from other extensions
- `footerData.onBranchChange(callback)` — subscribe to branch changes, returns unsubscribe function
- Token/cost stats: iterate `ctx.sessionManager.getBranch()`, sum up from assistant messages (`entry.message.usage`)
- Model info: `ctx.model?.id`, thinking level from `pi.getThinkingLevel()`
- Session name: `pi.getSessionName()`
- `ctx.cwd` for current working directory
- Use `truncateToWidth` and `visibleWidth` from `@mariozechner/pi-tui`
- The footer should activate automatically on `session_start` (no command needed to toggle)
- Use `footerData.onBranchChange()` for reactive updates

## Reference

- The example extension at the path below demonstrates the `setFooter` API:
  `/nix/store/mws8dqyglrbr9ljhxk0iabaim63c4gqs-pi-0.55.0/lib/node_modules/@mariozechner/pi-coding-agent/examples/extensions/custom-footer.ts`
- Extension docs:
  `/nix/store/mws8dqyglrbr9ljhxk0iabaim63c4gqs-pi-0.55.0/lib/node_modules/@mariozechner/pi-coding-agent/docs/extensions.md`
- TUI docs:
  `/nix/store/mws8dqyglrbr9ljhxk0iabaim63c4gqs-pi-0.55.0/lib/node_modules/@mariozechner/pi-coding-agent/docs/tui.md`
