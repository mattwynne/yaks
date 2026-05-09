# Yaks (`yx`)

> It is in the doing of the work that we discover the work that we must do
>
> -- Woody Zuill, https://agilemaxims.com

Yaks is a tiny, fast yak map for software teams: a command-line way to capture the work you discover while doing the work.

A yak map is a tree of nested goals. Put prerequisites underneath the larger goal they unlock, and the map shows what can happen now, what can happen in parallel, and what is still blocked.

Yaks is built around three values:

- **Simple**: everything is a yak. No separate epics, stories, tasks, bugs, or chores. Just multi-word names, optional context, tags, and fields when your team needs them. The core flow is `todo` → `wip` → `done`, with `blocked` available when work is explicitly waiting.
- **Collaborative**: yaks sync through git with conflict-free event merging. Multiple people and coding agents can update the same map from different branches, clones, and worktrees without coordinating edits.
- **Delightful**: the CLI should feel instant (<100ms for everyday operations), forgiving, and pleasant: fuzzy matching, tab completion, multi-word names, JSON output, and a pretty tree.

It's designed for humans and AI coding agents working together in the same codebase, without adding a server, database, or heavyweight process.

![demo](demo.gif)

## Install

### macOS/Linux

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | bash
```

The installer downloads the latest stable GitHub Release, validates the checksum when available, installs the `yx` binary in `/usr/local/bin`, and installs shell completions so you can tab-complete yak names.

To install a specific release:

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | YX_VERSION=1.2.3 bash
```

To install the edge channel:

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | YX_CHANNEL=edge bash
```

## Quick start

Run these commands from a git repository:

```bash
yx add "Fix the bug"                     # Add a yak
yx add "Write a failing test" --under "Fix the bug"
yx context "Fix the bug"                 # Add notes/context
yx list                                  # Show the tree
yx show "Fix the bug"                    # Show yak details
yx start "Write a failing test"          # Mark as work-in-progress
yx done "Write a failing test"           # Mark as complete
yx sync                                  # Sync with teammates
yx remove "Fix the bug"                  # Remove a yak
yx prune                                 # Remove all done yaks
```

Children block their parent: put prerequisites underneath the larger goal. That way the map shows what can be done now, what can happen in parallel, and what is waiting on something else.

## For AI coding agents

Yaks does not need a special agent integration. Agents can use the same CLI as humans:

- `yx list` / `yx show` to discover work and read context
- `yx start` to claim work
- `yx add` / `yx context` to record discoveries
- `yx done` to finish work
- `yx sync` to share updates
- `yx list --format json` and `yx show --format json` for structured output

Tell your coding agent that this repo uses yaks. For example:

```bash
echo 'This project uses yaks (yx) for task management. See !yx help' > CLAUDE.md
```

If your agent reads `AGENTS.md`, you can add the same line there too:

```bash
echo 'This project uses yaks (yx) for task management. See !yx help' > AGENTS.md
```

Multiple agents can update the yak map at the same time. Yaks uses an event-sourced CRDT-style merge on a hidden git ref, so simultaneous updates sync without normal file merge conflicts.

## Why "Yaks"?

The name comes from [yak shaving](https://en.wiktionary.org/wiki/yak_shaving) — when you set out to do task A but discover you need B first, which requires C. A Yak Map captures this emergent structure as a tree.

It's the same idea as a [Mikado Graph](https://mikadomethod.info) or a [Discovery Tree](https://www.fastagile.io/method/product-mapping-and-discovery-trees), but I like calling it a Yak Map, because yak shaving is what we do all day in software.

![image](https://github.com/user-attachments/assets/1e935831-7807-4127-a698-3fdb50615080)

## Related tools and trade-offs

Yaks is not trying to be the most complete issue tracker. It is trying to be a sharp, dependency-first planning tool that stays light enough for a team to use continuously while coding.

### Beads

[Beads](https://github.com/steveyegge/beads) is a comprehensive issue tracker and agent orchestration system. It offers a rich task schema, many dependency types, database-backed sync, workflow features, and integrations for managing agents.

Yaks comes from XP, mob programming, and collaborative planning: make the emerging dependency map visible, keep the model small, and trust the team to self-organise. If you want a capable orchestration platform, Beads is worth a look. If you want a tiny CLI that keeps everyone oriented during the work, Yaks is deliberately smaller.

### git-bug

[git-bug](https://github.com/git-bug/git-bug) is a distributed, offline-first issue tracker that stores issues, comments, identities, and history as git objects in the repository. It can sync through git remotes, offers CLI/TUI/web interfaces, and can bridge to systems like GitHub and GitLab.

Yaks also uses git for sharing, but it is not a GitHub-Issues-style tracker. It optimises for dependency-first yak maps, lightweight CLI workflow, agent-friendly tree/context output, and CRDT-style collaboration through git event sync. Choose git-bug when you want full issue tracking inside git; choose Yaks when the tree of discovered prerequisites is the central artefact.

### kata

[kata](https://github.com/wesm/kata) is a local-first issue ledger for humans and coding agents. It emphasises stable agent commands, JSON output, predictable failure modes, a TUI for human oversight, and auditability through events. Its current architecture uses a local daemon and SQLite store, with future shared-server collaboration planned.

Yaks shares kata's interest in human/agent collaboration and a small complexity budget, but makes different choices: project-local yak maps, git-backed sharing today, no daemon, and a tree-shaped model where dependency structure is primary. kata is a good fit when you want a durable issue ledger with comments, labels, ownership, and a TUI; Yaks is a good fit when you want the fastest possible shared map of what is blocking what.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, testing, and mutation testing instructions.

## License

[MIT](LICENSE)
