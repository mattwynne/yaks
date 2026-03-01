---
name: exploratory-testing
description: Use when a feature feels under-tested, after implementing new functionality, or before a release to discover edge cases, UX issues, and bugs through hands-on CLI exploration
---

# Exploratory Testing

## Overview

**Exploratory testing discovers what automated tests miss.** The agent
acts as a curious, methodical user - running `yx` commands in a
sandbox, observing output, trying edge cases, and logging findings.
Each session is guided by a charter that focuses exploration on a
specific area with specific heuristics.

## When to Use

- After implementing a new feature or fixing a bug
- Before a release, to shake out edge cases
- When a command's behaviour feels under-tested
- When you want to stress-test error handling or output formatting
- Periodically, to discover regressions or UX papercuts

**Don't use when:**
- You need deterministic, repeatable test coverage (write Cucumber scenarios)
- The area has no working implementation yet

## Phase 1: Charter

### Agree on a Target

The user provides (or the agent suggests) a target area to explore.
Good targets are specific commands, workflows, or quality attributes:

- "the `yx add` command"
- "yak hierarchy and the `--under` flag"
- "output formatting across `--format` options"
- "error messages when things go wrong"
- "a full workflow: add, organise, work, complete, prune"

### Select Heuristics

Pick 2-4 heuristics from the menu below that suit the target:

| Heuristic | CLI Application |
|-----------|----------------|
| **CRUD** | Add, list, show, update, remove yaks through full lifecycle |
| **Zero, One, Many** | Empty state, single yak, many yaks, deep nesting |
| **Boundary Values** | Long names, special chars, spaces, empty strings, unicode |
| **Never and Always** | Invariants (done yaks always show done, removed yaks never listed) |
| **Follow the Data** | Add -> list -> modify -> list -> verify consistency |
| **Some, None, All** | Filters with matching/non-matching/all items |
| **Starve** | Missing YAK_PATH, non-existent directories, no permissions |
| **Interrupt** | Broken pipes (`yx ls \| head -1`), partial stdin, Ctrl-C |
| **Configuration Tour** | `--format` options, `--only` filters, env vars |
| **Claims Tour** | Does `--help` text match actual behaviour? |
| **Sequence Variation** | Unusual command orders (done before add, prune with no done) |
| **User Tour** | Common real-world workflows end-to-end |

### Write the Charter

Format the charter using Elisabeth Hendrickson's template:

> **Explore** [target area]
> **With** [selected heuristics]
> **To Discover** [risks or information we seek]

Example:
> **Explore** the `yx add` command
> **With** Boundary Values, Zero/One/Many, Claims Tour
> **To Discover** how it handles edge-case input and whether
> `--help` accurately describes its behaviour

### Approval Gate

Present the charter to the user. **Do not proceed until the user
confirms the charter.** The user may adjust the target, heuristics,
or risk focus.

## Phase 2: Exploration

### Set Up the Sandbox

Create an isolated environment so exploration never touches the
project's real `.yaks/` data.

Follow the [yx-sandbox skill](../yx-sandbox/SKILL.md) to set up
and use a temp directory. Use the sandbox prefix on **every** `yx`
command during exploration.

### Seed the Environment

Before exploring, create enough data to work with:

```bash
# Using the literal sandbox path from mktemp output:
YAK_PATH=/tmp/tmp.xYz123AbC YX_SKIP_GIT_CHECKS=1 yx add make the tea
YAK_PATH=/tmp/tmp.xYz123AbC YX_SKIP_GIT_CHECKS=1 yx add buy biscuits
YAK_PATH=/tmp/tmp.xYz123AbC YX_SKIP_GIT_CHECKS=1 yx add wash the cups --under make the tea
YAK_PATH=/tmp/tmp.xYz123AbC YX_SKIP_GIT_CHECKS=1 yx state wash the cups wip
YAK_PATH=/tmp/tmp.xYz123AbC YX_SKIP_GIT_CHECKS=1 yx done buy biscuits
```

Adapt seeding to the charter - if exploring hierarchy, create deeper
nesting; if exploring empty state, skip seeding entirely at first.

### Explore Systematically

Work through each chartered heuristic. For each one:

1. **State what you're testing** (the heuristic and specific probe)
2. **Run the command** with the env var prefix
3. **Record the result** in your session log

Keep a running session log in this format:

```
### [Heuristic Name]

**Probe:** [what you're trying]
**Command:** `YAK_PATH=<sandbox-path> YX_SKIP_GIT_CHECKS=1 yx ...`
**Expected:** [what you thought would happen]
**Actual:** [what actually happened]
**Verdict:** OK | BUG | UX-ISSUE | INCONSISTENCY | UNEXPECTED | QUESTION
**Notes:** [any additional observations]
```

### Exploration Guidelines

- **Stay within the charter scope.** If you discover something
  interesting outside scope, note it for a future session.
- **Vary your inputs.** Don't just test happy paths.
- **Pay attention to output formatting.** Alignment, colours,
  whitespace, and truncation are all worth probing.
- **Check exit codes.** `echo $?` after commands that should
  fail - do they return non-zero?
- **Try composition.** Pipe output to other commands, use
  `--format plain` for scripting.
- **Time-box yourself.** 15-30 minutes of exploration per
  charter is usually enough.

## Phase 3: Report

### Present Findings

After exploration, present a structured report to the user.

#### Summary

One-line summary: how many probes, how many findings, overall
impression.

#### Findings by Category

Group findings into these categories (skip empty ones):

| Category | Description |
|----------|-------------|
| **Bugs** | Incorrect behaviour, crashes, wrong exit codes |
| **UX Issues** | Confusing output, unclear errors, surprising defaults |
| **Inconsistencies** | Behaviour differs between similar commands |
| **Missing Error Handling** | No error where one is expected, unhelpful messages |
| **Unexpected Behaviour** | Works but not how a user would expect |
| **Worked Well** | Things that behaved exactly right, good UX moments |

For each finding, include:
- The command that triggered it
- What happened vs what was expected
- Severity estimate (low / medium / high)

#### Suggested Follow-Ups

Concrete next steps:
- Yaks to create for bugs or improvements
- Cucumber scenarios to write for discovered edge cases
- Documentation to update if `--help` is misleading
- Areas worth a dedicated ET session

### Clean Up

Remove the sandbox as described in the
[yx-sandbox skill](../yx-sandbox/SKILL.md).

## Quick Reference

| Phase | What Happens | Gate |
|-------|-------------|------|
| Charter | Agree target, select heuristics, write charter | User approves charter |
| Exploration | Sandbox setup, systematic probing, session log | None - explore autonomously |
| Report | Categorised findings, follow-ups, cleanup | User reviews findings |

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Exploring without a charter | Always agree on target and heuristics first |
| Only testing happy paths | Heuristics exist to push beyond the obvious |
| Logging only failures | Record successes too - they confirm expected behaviour |
| Exploring everything at once | Pick 2-4 heuristics per session, stay focused |

| Skipping exit code checks | `echo $?` after commands that should fail |
| Not seeding enough data | Adapt seed data to what the charter needs |

## Sources

- Elisabeth Hendrickson: *Explore It!* (Pragmatic Bookshelf)
- James Bach: Session-Based Test Management
- Michael Bolton: "Testing vs. Checking"
