# Yaks (`yx`)

> It is in the doing of the work that we discover the work that we must do
>
> -- Woody Zuill, https://agilemaxims.com

Yaks is a friendly command-line TODO list for software teams. It stores work as a tree of nested goals — a _yak map_ — so you can capture the real shape of the work as you discover it.

It's designed for humans and AI coding agents working together in the same codebase. The yak map syncs through git, so everyone can update the plan from any branch, clone, or worktree without merge conflicts or extra infrastructure.

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

Tell your coding agent that this repo uses yaks. For example:

```bash
echo 'This project uses yaks (yx) for task management. See !yx help' > CLAUDE.md
```

If your agent reads `AGENTS.md`, you can add the same line there too:

```bash
echo 'This project uses yaks (yx) for task management. See !yx help' > AGENTS.md
```

Agents can then use normal `yx` commands to discover work, claim it, add notes, complete it, and sync with everyone else. Multiple agents can update the yak map at the same time; yaks uses an event-sourced CRDT merge so those updates do not conflict.

## Principles

**Simple.** Everything is a yak. No epics, stories, tasks, bugs, or chores. No priority fields, assignees, or estimates. Three states: todo, wip, done. Use tags and custom fields to model your own workflow.

**Collaborative.** Yaks uses event sourcing on a hidden git ref. Changes from any branch, clone, or worktree merge automatically — no conflicts, no coordination, no extra infrastructure. If you can `git push`, you can share yaks.

**Delightful.** The ergonomics are designed for both humans and agents. Robots get forgiving command and argument aliases for clumsy assumptions, plus JSON output as standard. Humans get a thoughtful, responsive UX with fuzzy name matching, tab completion, and useful views of the map.

## Why "Yaks"?

The name comes from [yak shaving](https://en.wiktionary.org/wiki/yak_shaving) — when you set out to do task A but discover you need B first, which requires C. A Yak Map captures this emergent structure as a tree.

It's the same idea as a [Mikado Graph](https://mikadomethod.info) or a [Discovery Tree](https://www.fastagile.io/method/product-mapping-and-discovery-trees), but I like calling it a Yak Map, because yak shaving is what we do all day in software.

![image](https://github.com/user-attachments/assets/1e935831-7807-4127-a698-3fdb50615080)

## How is this different from Beads?

[Beads](https://github.com/steveyegge/beads) is a powerful issue tracker built for AI agents. It has 81 fields per task, 19 dependency types, a SQL/Dolt database, and workflow templates. If you want a comprehensive system for orchestrating agents, it's impressive.

Yaks takes the opposite approach. It's a sharp, simple tool that trusts teams to self-organise:

- **One concept**: everything is a yak, nested under other yaks
- **Conflict-free sync**: event-sourced CRDT merge means multiple people and agents can update the yak map simultaneously without coordination
- **Zero infrastructure**: no database server, no config files, no lock files — just git

Yaks grew out of years of practice with XP, mob programming, and collaborative planning on human teams. Beads grew out of the single-user-multi-agent workflow. Different roots, different trade-offs.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, testing, and mutation testing instructions.

## License

[MIT](LICENSE)
