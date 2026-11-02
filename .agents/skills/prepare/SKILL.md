---
name: prepare
description: Prepare a yak for contributors maintaining Yaks itself through scope discussion, example mapping, BDD formulation, and implementation-ready context.
---

# Prepare a contributor yak

This workflow is for maintaining this repository. General guidance on using
Yaks and mapping prerequisites belongs in `yx --help` and command help.
Use this skill when a tracked yak needs requirements or examples clarified
before implementation. Skip phases already agreed with the human.

## Read the existing work

Read `AGENTS.md`, `CONTRIBUTING.md`, the yak's context, its parent and children,
and the relevant code and tests. Use `yx show "yak name" --format json`
and `yx list --format json`; inspect optional custom fields before trying
to read them. A missing `examples` field is normal for an unprepared yak.

Resolve existing questions with the human. Use `yx start "yak name"` to
claim the preparation work; this changes state, not the Git worktree.
Read `src/README.md` for changes to Rust code and
`docs/cli-design-philosophy.md` for CLI changes (paths are repository-relative).

## Agree scope and decisions

Discuss the goal, why it matters, what is in and out of scope, and how the
human will know it is done. Ask about decisions that change the outcome;
use judgment for routine details. Preserve answers already given rather
than asking the same questions again.

Identify the kind of change: observable behavior, refactoring, or chore.
Store the agreed goal, scope, acceptance criteria, decisions and reasons
in the yak's context with `yx context "yak name"` reading from stdin.
Read existing context first and preserve useful information when updating it.

## Discover behavior with example mapping

For observable behavior, read [BDD discovery](../bdd-discovery/SKILL.md).
List the behavioral rules briefly, then explore one rule at a time with
the human. Offer concrete examples, including important edge cases; agree
the rule and examples before moving on. Keep design choices such as layout
constants in the specification rather than inventing behavioral rules.

Capture open questions and discuss them one at a time. Record deferred
questions explicitly and identify which work they block. Slice newly
discovered behavior out when it would make the current yak too large.

Store the agreed rules, examples and outstanding questions in the yak's
`examples` field through stdin:

```bash
yx field "yak name" examples <<'EOF'
Rule: Children are prerequisites of their parent
Example: A goal with an unfinished child cannot be marked done
Example: A goal whose children are done can be marked done
EOF
```

Read [BDD formulation](../bdd-formulation/SKILL.md) to turn agreed examples
into concrete, readable Gherkin. Check existing `features/*.feature` and
step definitions so examples fit the project's language. Capture the agreed
scenarios on the yak for implementation; the work skill introduces one
failing scenario or test at a time, as required by `AGENTS.md`.

## Refactorings and chores

For refactorings, agree what must remain unchanged and why the structure
should change. For architectural decisions, read
[CQRS and event sourcing](../cqrs-event-sourcing/SKILL.md) when applicable
and write a new proposed ADR using `docs/adr/adr_template.md`.
Never edit an accepted ADR. Record the ADR path on the yak.

Chores usually need a clear goal, scope and verification criteria rather
than example mapping. Preserve the project's TDD requirements when a chore
changes executable behavior.

## Leave implementable work

The yak hierarchy is the plan; do not require a separate `plan` field or
write a parallel planning document. Consult `yx add --help`, `yx move --help`
and `yx blocker add --help` for mapping and dependency guidance.

Split work only when needed for independently understandable changes.
For each child, record its goal, relevant file paths, agreed examples or
acceptance criteria, verification approach, dependencies and known questions.
Show the updated map after each addition or move so the human can follow it.

If an unanswered question prevents implementation, record it in context
and use `yx tag add "yak name" @needs-human`. Remove the tag with
`yx tag rm "yak name" @needs-human` once answered. An informational tag
does not replace a real blocker; use `yx blocker add --help` when work is
actually blocked, and remove the blocker when resolved.

Confirm the resulting scope, examples and work structure with the human.
Once preparation is complete, leave executable leaves in `todo` with
`yx state "yak name" todo` so `yx list --ready` can find them.
Hand off to [work](../work/SKILL.md) when implementation is requested.
