# Yaks - An iterative, emergent, non-linear TODO list for humans and robots

> It is in the doing of the work that we discover the work that we must do
>
> -- Woody Zuill, https://agilemaxims.com

Yaks is a command-line tool for managing a shared TODO list as a tree
of nested goals. It's designed for teams — humans and AI agents
working together on the same codebase.

Everyone on the team works from the same yak map. Changes sync
through git with zero merge conflicts, so you never have to
coordinate who's updating the plan.

![demo](demo.gif)

## Principles

**Simple.** Everything is a yak. No epics, stories, tasks, bugs, or
chores. No priority fields, no assignees, no estimations. Three
states: todo, wip, done. If you need more, add a custom field. You
probably don't need more.

**Collaborative.** Yaks uses event sourcing on a hidden git ref.
Changes from any branch, clone, or worktree merge automatically —
no conflicts, no coordination, no extra infrastructure. If you can
`git push`, you can share yaks.

**Delightful.** Every command completes in under 100ms. Multi-word
names without quotes. Fuzzy matching so you don't need exact names.
A pretty tree view. Tab completion. Small things that add up.

## Usage

```bash
yx add Fix the bug                     # Add a new yak
yx add Buy milk --under Fix the bug    # Nest under a parent
yx context Fix the bug                 # Add context/notes
yx show Fix the bug                    # Show yak details
yx ls                                  # Show the tree
yx state Fix the bug wip               # Mark as work-in-progress
yx done Fix the bug                    # Mark as complete
yx sync                                # Sync with teammates
yx rm Fix the bug                      # Remove a yak
yx prune                               # Remove all done yaks
```

## Why "Yaks"?

The name comes from [yak shaving](https://en.wiktionary.org/wiki/yak_shaving)
— when you set out to do task A but discover you need B first, which
requires C. A Yak Map captures this emergent structure as a tree.

It's the same idea as a
[Mikado Graph](https://mikadomethod.info) or a
[Discovery Tree](https://www.fastagile.io/method/product-mapping-and-discovery-trees),
but I like calling it a Yak Map, because yak shaving is what we do
all day in software.

![image](https://github.com/user-attachments/assets/1e935831-7807-4127-a698-3fdb50615080)

## For AI coding agents

Add instructions to your `CLAUDE.md` or `AGENTS.md` telling your
agent to use `yx` commands. No MCP server, no plugins, no special
integration — just a CLI.

```bash
yx ls --format json          # Discover work
yx state "fix the bug" wip   # Claim a yak
echo "notes" | yx field "fix the bug" progress  # Store notes
yx done "fix the bug"        # Complete work
yx sync                      # Sync with teammates
```

Multiple agents can update the yak map simultaneously.
The event-sourced CRDT merge means their changes never conflict —
they just interleave.

## How is this different from Beads?

[Beads](https://github.com/steveyegge/beads) is a powerful issue
tracker built for AI agents. It has 81 fields per task, 19
dependency types, a SQL database, and workflow templates. If you
want a comprehensive system for orchestrating agents, it's
impressive.

Yaks takes the opposite approach. It's a sharp, simple tool that
trusts teams to self-organise:

- **One concept**: everything is a yak, nested under other yaks
- **Conflict-free sync**: event-sourced CRDT merge means multiple
  people and agents can update the yak map simultaneously without
  coordination
- **Zero infrastructure**: no database server, no config files,
  no lock files — just git

Yaks grew out of years of practice with XP, mob programming, and
collaborative planning on human teams. Beads grew out of the
single-user-multi-agent workflow. Different roots, different
trade-offs.

## Installation

### Quick Install (macOS/Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/mattwynne/yaks/main/install.sh | bash
```

The installer downloads the latest release, validates the checksum
and installs the binary in `/usr/local/bin`. It also installs shell
completions so you can tab-complete yak names.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup,
testing, and mutation testing instructions.

## License

[MIT](LICENSE)
