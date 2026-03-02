---
name: triaging-mutants
description: Use when mutation testing reveals survived mutants — guides deep analysis of whether each mutant signals a missing test, a design improvement opportunity, or an equivalent mutation
---

# Triaging Mutants

## Overview

**A survived mutant is a question, not just a gap in test
coverage.** When `cargo mutants` reports a miss, the obvious
response is to write a test that kills it. But that's often the
shallow fix. The deeper question is: *why did this mutation
produce code that still works?*

Three possibilities:

1. **Missing test** — the behaviour is real and untested.
   Write a test.
2. **Design weakness** — the code tolerates the mutation
   because the design is too permissive, has redundant paths,
   or uses a representation that doesn't constrain values
   tightly enough. Improve the design so the mutation becomes
   impossible or obviously wrong.
3. **Equivalent mutant** — the mutation produces logically
   identical behaviour. No action needed; mark it accepted.

Option 2 is the most valuable and most commonly overlooked.
Changing the modelling often obviates entire classes of
mutants — making the code both more correct and simpler to
test.

## When to Use

- After a mutation test run produces missed mutants
- When working on a "fix missed mutants" sub-yak
- When reviewing mutation results before deciding what to do
- Periodically, to reassess previously-accepted mutants

**Don't use when:**
- No mutation results exist yet (run `dev mutate` first)
- The mutant is in generated or trivial code (skip it)

## Pacing

**Don't ask which mutant to triage next — just pick one and
go.** After presenting a verdict and acting on it, immediately
move to the next untriaged mutant. Keep momentum. The user
will interrupt if they want to change direction.

Prefer domain-layer mutants over adapter-layer, and
single-mutant files over multi-mutant files (quick wins
build momentum).

## Inputs

The skill expects one of:

- A specific mutant yak (e.g. `yx show "adapters-event_store-mod mutants"`)
- A file path with mutants from `mutants.out/missed.txt`
- The full `mutants.out/missed.txt` for a triage session

## Phase 1: Gather Context

For each mutant (or group of mutants in the same file):

1. **Read the mutant description** from the yak context or
   `missed.txt`:
   ```
   src/domain/yak_map.rs:277:59: replace && with || in YakMap::update_state
   ```

2. **Read the source code** around the mutation site. Read
   enough context to understand the function's purpose — not
   just the mutated line, but the whole function and its
   callers.

3. **Read existing tests** that exercise this code path.
   Search for tests that call the function, Cucumber scenarios
   that exercise the behaviour, and any test helpers involved.

4. **Understand the domain intent.** What business rule or
   invariant does this code enforce? What would go wrong in
   the real world if this mutation were live?

## Phase 2: Classify Each Mutant

For each mutant, work through this decision tree:

```
Is the mutated code reachable from tests?
├─ No → MISSING TEST (coverage gap)
│       But ask: why is it unreachable? Dead code?
╰─ Yes
   Does the mutation change observable behaviour?
   ├─ No → EQUIVALENT MUTANT
   │       The mutation is semantically identical.
   │       Document why and mark accepted.
   ╰─ Yes
      Do existing tests exercise the affected behaviour?
      ├─ No → MISSING TEST (specific scenario gap)
      ╰─ Yes
         Why don't they catch it?
         ├─ Assertions are too weak → MISSING TEST
         │  (tighten assertions or add a specific case)
         ╰─ The design tolerates both variants
            → DESIGN WEAKNESS
```

### Classification: Missing Test

The behaviour is real and untested. Before writing the test,
ask:

- **What rule does this test express?** Name the business rule
  or invariant, not the implementation detail. A test called
  "merge prefers higher event count" is better than "line 111
  uses plus not minus".
- **Where does it belong?** Cucumber scenario (if it's
  user-observable behaviour), unit test (if it's an internal
  invariant), or integration test (if it's adapter-specific)?
- **Is the test worth its maintenance cost?** Most are. But if
  the code is genuinely trivial (a builder method that sets a
  field), consider whether the mutant is noise.

### Classification: Design Weakness

This is the high-value finding. The mutation survived because
the design permits it. Common patterns:

| Pattern | Example | Better Design |
|---------|---------|---------------|
| **Boolean blindness** | `if flag && other_flag` — swapping `&&`/`||` still works | Replace booleans with an enum that names the states |
| **Primitive obsession** | Using `i32`, `String`, `bool` where a domain type would constrain meaning | Introduce a newtype, enum, or value object that makes invalid states unrepresentable |
| **Conditional logic** | `if`/`else` or `match` chains selecting behaviour — flipping conditions still works | Replace with strategy pattern, polymorphism, or trait objects so the type system enforces the dispatch |
| **Stringly-typed data** | String manipulation where a type would constrain values | Introduce a newtype or enum |
| **Redundant paths** | Two code paths that happen to produce the same result | Unify the paths or make them clearly distinct |
| **Weak preconditions** | Function accepts broad input, only uses narrow range | Narrow the input type or add validation |
| **Arithmetic on indices** | `+` vs `-` doesn't matter because ranges overlap | Use named operations or iterator methods |
| **Match arm equivalence** | Deleting a match arm still works because the default covers it | Make the match exhaustive and meaningful |
| **Dead code** | Code exists but nothing exercises it | Remove it |

**When you identify a design weakness, stop and discuss it
with the user.** Design changes have broader implications than
adding a test. Present:

1. **The mutant** — what survived and where
2. **The current design** — what permits the mutation
3. **The proposed improvement** — how the design could change
4. **The trade-off** — what the change costs (complexity,
   refactoring scope, risk)

Wait for the user to decide: improve the design now, defer it
to a new yak, or fall back to a test.

### Classification: Equivalent Mutant

The mutation doesn't change behaviour. This is rare but real.
Examples:

- Replacing `>` with `>=` when the boundary value never occurs
- Changing arithmetic on a value that's always zero
- Reordering commutative operations

**Be sceptical of this classification.** Most "equivalent"
mutants are actually design weaknesses in disguise — the
boundary value *should* occur, or the code *should* handle
it.

To mark as accepted, propose adding a `#[mutants::skip]`
annotation with a comment explaining why:

```rust
#[mutants::skip] // Equivalent: migration version is never 0 in practice
fn source_version() -> u32 { 4 }
```

Always discuss with the user before marking equivalent.

## Phase 2 Output: Triage Verdict

**Every mutant analysis MUST end with a clear verdict block.**
This is the deliverable — it tells the user what you found
and what you recommend, in a format that supports quick
decisions.

```
## Triage Verdict

**Mutant:** `<mutation description>` in `<file>:<line>`
**Classification:** Missing Test | Design Weakness | Equivalent Mutant
**Why it survives:** <1-2 sentence explanation of the mechanism>
**Recommendation:** <what to do — write a test, refactor, annotate>
**Test level:** Cucumber scenario | Unit test | Integration test
**Effort:** Small (1 assertion) | Medium (new test) | Large (refactoring)

### Questions (if any)
- <open question for the user>
```

For design weaknesses, add:

```
### Design Analysis
**Current design:** <what permits the mutation>
**Proposed improvement:** <how to fix it>
**Trade-off:** <cost vs benefit>
```

Group multiple mutants in the same file into a single verdict
when they share the same root cause.

## Phase 3: Act

For each classified mutant, take the appropriate action:

### Missing Test → Write It

Follow TDD:
1. Write the failing test first (Cucumber scenario or Rust
   test)
2. Verify it fails — ideally by applying the mutation manually
   and confirming the test catches it
3. Commit with a message referencing the mutant:
   ```
   Add test for [rule] — kills mutant in [file]:[line]
   ```

### Design Weakness → Propose and Discuss

1. **Present the analysis** to the user (see Phase 2)
2. If approved for immediate fix:
   - Make the design change
   - Verify existing tests still pass
   - Verify the mutant is now killed (or impossible)
   - Commit
3. If deferred:
   ```bash
   yx add "improve [description]" --under "[parent yak]"
   cat <<'EOF' | yx context "improve [description]"
   # Motivation
   Mutant survived: [mutant description]

   # Current Design
   [What permits the mutation]

   # Proposed Change
   [How to improve it]
   EOF
   ```

### Equivalent Mutant → Annotate

1. Add `#[mutants::skip]` with explanation
2. Commit with message explaining the equivalence

## Phase 4: Report

After triaging all mutants in the session, summarise:

```
## Triage Summary

| Category | Count | Actions Taken |
|----------|-------|---------------|
| Missing test | N | N tests written |
| Design weakness | N | N fixed, N deferred |
| Equivalent | N | N annotated |

### Design Improvements Identified
- [Brief description of each]

### Deferred Items
- [Yaks created for later]
```

Update the mutant yak's state:
- All mutants addressed → `yx done "yak name"`
- Some deferred → update context with what remains

## Evolving This Skill

This skill grows through use. When a triage session produces a
new insight — a design weakness pattern we haven't seen before,
a classification heuristic that proved useful, or a mistake we
learned from — **update this skill file before closing the
session.**

Specifically:
- New design weakness patterns → add a row to the table in
  Phase 2
- New equivalent mutant justifications → add to the examples
- Recurring mistakes → add to Common Mistakes
- Better questions to ask during classification → refine the
  decision tree

The goal is that every triage session leaves the skill slightly
better for the next one.

## Quick Reference

| Phase | What | Gate |
|-------|------|------|
| Gather | Read mutant, source, tests, domain intent | — |
| Classify | Decision tree → missing test / design / equivalent | — |
| Verdict | Present findings, recommendation, questions | User decides |
| Act (test) | Write test, verify it kills mutant | Tests pass |
| Act (design) | Present analysis, discuss with user | User approves |
| Act (equivalent) | Annotate with skip + reason | User approves |
| Report | Summary table, update yak state | — |

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Immediately writing a test without analysing why the mutant survived | Always classify first — the test might be the wrong fix |
| Classifying a design weakness as "missing test" | If `&&`→`||` survives, ask why both work, not just "add a test for &&" |
| Treating all mutants as independent | Group by file/function — patterns often emerge |
| Making design changes without discussing | Design weakness → always present to user first |
| Marking mutants as equivalent too readily | Most "equivalents" are design weaknesses in disguise |
| Writing tests that test the implementation, not the behaviour | Name the business rule, not the code line |
| Skipping the domain intent step | Understanding *why* the code exists is the whole point |
| Presenting analysis without a clear verdict | Always end with the Triage Verdict block — findings, recommendation, questions |
