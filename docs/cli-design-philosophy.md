# yx CLI Design Philosophy

This document captures the design principles that guide yx's command-line interface. These principles are informed by modern CLI best practices (clig.dev, 12 Factor CLI Apps) while staying true to yx's core mission: a simple, composable tool for humans and robots working together.

## Core Philosophy

**Simple tools that do one thing exceptionally well.**

yx manages hierarchical TODO lists (Yak Maps). It doesn't track time, set priorities, send notifications, or integrate with external systems. This laser focus allows us to optimize the interface for the core workflow.

**Designed for both humans and robots.**

yx is used by human developers AND AI agents working simultaneously on the same repository. Every decision considers both audiences:
- Humans need readability, colors, helpful messages
- Robots need parseable output, consistent patterns, composability

## Design Principles

### 1. Ergonomics First

**Make the common case easy.** The CLI should feel invisible - users shouldn't have to think about syntax.

**Concrete Examples:**

```bash
# Multi-word names without quotes (natural language)
yx add Fix authentication bug in login flow
yx done Fix authentication bug in login flow

# Short, memorable aliases for frequent operations
yx ls              # instead of always typing "list"
yx rm "old task"   # instead of "remove"

# Sensible defaults
yx list            # pretty format with colors (humans)
yx list --format plain  # machine-readable (robots)

# Context from stdin for scripting
echo "## Notes" | yx add "my task"
```

**Why it matters:** CLI tools are used hundreds of times per day. Every keystroke saved, every quote mark avoided, compounds into significant efficiency gains.

**Anti-pattern:** Requiring quotes for common input, verbose flag names for frequent operations, no aliases for long commands.

### 2. Discoverability & Help

**Users should be able to learn the tool without leaving the terminal.**

**Concrete Examples:**

```bash
# Basic help shows all commands
yx --help

# Command-specific help with examples
yx add --help

# Alias discovery (ls works for list)
yx ls --help  # Shows "list" command help

# Tab completion for yak names
yx done <TAB>  # Shows all incomplete yaks
```

**Help text structure:**
- Brief description of what the command does
- Required vs optional arguments
- Available flags with clear descriptions
- Output format options explained

**Why it matters:** Good help reduces support burden and makes the tool accessible to newcomers. Tab completion makes the tool faster for everyone.

**Anti-pattern:** Help text that just lists flags without context, no examples, hidden features that require reading docs.

### 3. Clear, Actionable Feedback

**Tell users what happened, what went wrong, and how to fix it.**

**Concrete Examples:**

```bash
# Success: explain the change
$ yx done "Fix bug"
Marked 'Fix bug' as done

# Error: specific and actionable
$ yx done "Nonexistent task"
Error: yak 'Nonexistent task' not found

# Validation: explain why it failed
$ yx add "task:with:colons"
Error: Invalid yak name - cannot contain colons (:)

# Business logic: explain the constraint
$ yx done "parent task"
Error: cannot mark 'parent task' as done - it has incomplete children
Hint: use --recursive to mark all children as done
```

**Error message pattern:**
1. What went wrong (specific, not generic)
2. Why it matters (context if not obvious)
3. How to fix it (actionable next step)

**Why it matters:** Clear errors reduce frustration and support requests. Users learn the tool's rules through error messages.

**Anti-pattern:** Generic errors ("Error: operation failed"), no context, no suggestion for resolution.

### 4. Human & Machine Output Modes

**Default to human-friendly, provide machine-readable alternatives.**

**Concrete Examples:**

```bash
# Human mode: colors, formatting, helpful messages
$ yx list
- [todo] Fix authentication bug
  - [done] Research OAuth libraries
  - [todo] Implement token refresh

You have no yaks. Are you done?

# Machine mode: parseable, no colors, no decorations
$ yx list --format plain
Fix authentication bug
Fix authentication bug/Research OAuth libraries
Fix authentication bug/Implement token refresh

$ yx list --format plain --only not-done | wc -l
2
```

**Output format options:**
- `pretty`: Unicode, colors, visual hierarchy (default for TTY)
- `markdown`: Checkbox lists, indentation, some color
- `plain`/`raw`: Just names, one per line, full paths, no formatting

**Why it matters:** Humans need context and visual cues. Scripts need predictable, parseable output. Supporting both makes the tool universally useful.

**Anti-pattern:** Same output for humans and machines, decorative borders in tables, mixing output streams incorrectly.

### 5. Consistency & Convention

**Follow patterns users already know. Be internally consistent.**

**Concrete Examples:**

```bash
# POSIX conventions
yx --help          # Always available
yx list --format plain  # Long flags with double dash
yx done -r parent  # Short flags with single dash (future)

# Consistent aliasing pattern
yx list  / yx ls
yx remove / yx rm
yx move / yx mv
yx done / yx finish

# Consistent output streams
success → stdout
errors → stderr (with "Error:" prefix)
help → stderr (standard POSIX)

# Consistent exit codes
0 = success
1 = error (not found, validation failed, etc.)
```

**Why it matters:** Consistency reduces cognitive load. Users build a mental model of how the tool works and can predict behavior.

**Anti-pattern:** Inconsistent flag naming, errors to stdout, exit code 0 for failures, arbitrary patterns.

### 6. Composability & Scripting

**Play well with other tools. Support Unix pipeline patterns.**

**Concrete Examples:**

```bash
# Pipe-friendly output
yx list --format plain | grep "bug" | xargs -I {} yx done {}

# Stdin for context
echo "See issue #123" | yx add "Fix login bug"

# Exit codes for control flow
if yx done "Deploy to prod"; then
  notify-send "Deployment complete"
fi

# Filter and transform
yx list --format plain --only not-done | \
  fzf --prompt="Pick a yak: " | \
  xargs -I {} yx context --show {}
```

**Composability checklist:**
- ✅ Output goes to stdout, errors to stderr
- ✅ Exit codes indicate success/failure
- ✅ Plain format for machine parsing
- ✅ Stdin support for piping input
- ✅ No interactive prompts in non-TTY contexts
- ✅ Color detection (don't colorize when piped)

**Why it matters:** CLI tools are LEGO blocks. Composability multiplies power through combination.

**Anti-pattern:** Interactive prompts in scripts, mixing output streams, no machine-readable format, required flags for basic operations.

### 7. Validation & Safety

**Catch errors early. Validate input. Prevent data loss.**

**Concrete Examples:**

```bash
# Input validation at the boundary
$ yx add "task|with|pipes"
Error: Invalid yak name - cannot contain pipes (|)

# Character allowlist for filesystem safety
# Allowed: letters, numbers, spaces, hyphens, underscores, forward slash
# Rejected: \ : * ? | < > " (filesystem-unsafe)

# Business logic validation
$ yx done "parent with incomplete children"
Error: cannot mark 'parent' as done - it has incomplete children
Hint: use --recursive to mark all children as done

# Future: confirmation for destructive operations
$ yx prune
About to remove 5 done yaks. Continue? (y/N)
```

**Why it matters:** Early validation prevents corrupt state. Clear rules make the tool predictable.

**Anti-pattern:** Silently accepting invalid input, cryptic filesystem errors, no validation at boundaries.

### 8. Speed & Responsiveness

**Operations should feel instant. No unnecessary blocking.**

**Concrete Examples:**

```bash
# Fast operations (< 100ms)
yx add "task"          # Directory creation
yx list                # Directory scan
yx done "task"         # File write

# No network calls in core commands
# Sync is explicit and separate
yx sync                # Only command that touches network

# Minimal dependencies
# Pure Rust implementation, compiled binary
# No runtime dependencies beyond system libraries
```

**Why it matters:** Speed compounds. A 100ms delay becomes 10 seconds over 100 operations. Fast tools encourage frequent use.

**Anti-pattern:** Unnecessary network calls, slow startup, blocking operations, interpreting scripts on every invocation.

### 9. Progressive Disclosure

**Show simple things simply. Reveal complexity gradually.**

**Concrete Examples:**

```bash
# Basic usage is simple
yx add "task"
yx list
yx done "task"

# Advanced features available but not required
yx list --format markdown --only not-done
yx list --ready
yx blocker add "blocked task" --reason "waiting for credentials"
yx blocker add "blocked task" --by "blocking task" --reason "waiting on it"
yx done --recursive "parent task"
yx context "task" --show

# Help reveals more detail
yx --help              # Overview of commands
yx list --help         # Detailed format options
```

**Complexity layers:**
1. Core commands (add, list, done, rm) - visible in main help
2. Useful flags (--format, --only, --ready) - visible in command help
3. Advanced features (--recursive, stdin) - documented but optional

**Why it matters:** New users aren't overwhelmed. Power users discover advanced features gradually.

**Anti-pattern:** Exposing all options upfront, requiring advanced flags for basic operations, no learning path.

### 10. Delightful Details

**Small touches that make the tool pleasant to use.**

**Concrete Examples:**

```bash
# Friendly messages
$ yx list
You have no yaks. Are you done?

# Visual hierarchy with indentation
- [todo] parent task
  - [done] child task
    - [todo] grandchild task

# Color coding for status
✓ done tasks → grey (de-emphasized)
• todo tasks → white/default
• wip tasks → yellow (future)

# Sensible sorting
# Done tasks first, then alphabetical
# (Done tasks at top = sense of progress)
```

**Why it matters:** Delight turns users into advocates. Small touches show care and craft.

**Anti-pattern:** Sterile output, no personality, purely functional with no humanity.

## Non-Goals

**What yx intentionally does NOT do:**

- ❌ **Time tracking** - Use dedicated tools
- ❌ **Priority levels** - Use state or hierarchy
- ❌ **Rich text/formatting** - Markdown in context files
- ❌ **External integrations** - Use scripts + plain format
- ❌ **Authentication/permissions** - Use git/filesystem
- ❌ **Cloud sync** - Use git remotes
- ❌ **Interactive TUIs** - Stay composable with other tools
- ❌ **Configuration files** - Use environment variables

**Why this matters:** Feature creep kills simplicity. By clearly defining what we DON'T do, we protect the tool's core value: simplicity and composability.

## Design Decision Framework

When evaluating a new feature or change, ask:

1. **Does this serve both humans AND robots?**
   - If only humans: consider if scripting workaround exists
   - If only robots: ensure humans aren't harmed

2. **Does this make the common case easier?**
   - 90% of users → yes: probably good
   - 10% of users → no: probably feature creep

3. **Does this add complexity to the interface?**
   - New command: high bar (is it truly core?)
   - New flag: medium bar (is it commonly needed?)
   - New format option: low bar (enables new workflows)

4. **Is there a simpler way?**
   - Can composability solve this?
   - Can environment variables solve this?
   - Can documentation solve this?

5. **Does this follow our patterns?**
   - Consistent with existing commands?
   - Follows POSIX conventions?
   - Predictable based on mental model?

## References

This design philosophy draws from:

- [Command Line Interface Guidelines (clig.dev)](https://clig.dev/) - Modern CLI best practices
- [12 Factor CLI Apps](https://medium.com/@jdxcode/12-factor-cli-apps-dd3c227a0e46) - Heroku's CLI methodology
- [The Art of Command Line](https://github.com/jlevy/the-art-of-command-line) - What power users expect
- yx's core mission: simple, composable tools for humans and robots

## Living Document

This is a living document. As yx evolves, these principles should guide decisions while remaining open to refinement based on real-world usage.

When in doubt, favor:
- **Simplicity** over features
- **Consistency** over novelty
- **Speed** over completeness
- **Composability** over integration
- **Clarity** over cleverness
