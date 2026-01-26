# Incremental TDD

**ONE FAILING TEST AT A TIME**

## The Iron Law

You must write exactly one failing test, implement minimal code to pass it, then repeat. No exceptions.

This is not optional. This is not flexible. This is how TDD works.

## The Cycle

1. Write ONE simple test (start with the simplest possible case)
2. Run ALL tests - watch it fail (RED)
3. Write minimal code to pass that test (GREEN)
4. Run ALL tests - verify all pass
5. Refactor if needed:
   - Check if code quality can improve
   - Dispatch reviewer subagent with adr-review skill for ADR compliance
   - Fix any issues found
6. Run ALL tests - verify still passing after refactoring
7. Commit
8. Return to step 1

**Critical**: Always run ALL tests, not just the new one. You must catch regressions immediately.

## Two Stages of RED

When you write a test that doesn't compile:
1. **Stage 1 RED**: Compilation/syntax error - add the minimal stub (empty function, basic type)
2. Run tests again
3. **Stage 2 RED**: Test runs but fails - now you see the actual behavioral failure
4. **GREEN**: Implement the real behavior

Do not jump from compilation errors directly to full implementation.

## Why One Test At A Time Matters

The design emerges through the REFACTOR phase. Each passing test tells you what to build next.

If you write multiple failing tests first, you've already committed to a design without letting the tests guide you. You've skipped the core benefit of TDD.

## ADR Review in REFACTOR Phase

After tests pass (GREEN phase), dispatch a reviewer subagent with the `adr-review` skill to check if the code complies with architectural decision records (ADRs). This happens before committing.

The reviewer checks structural issues and naming conventions that tests don't verify.

Skip this only for trivial changes or if no ADRs exist.

## Red Flags (You're Doing Batch TDD)

- Writing multiple FAILING tests before implementing any
- Planning out all test cases then writing them together
- Jumping from a compile error to full implementation
- Writing 3-4 tests "just to cover the main scenarios"

## Common Rationalizations (All Wrong)

❌ "I need comprehensive test coverage upfront"
❌ "I'll write all the tests first then implement"
❌ "Just these 3-4 cases to start"
❌ "It's faster to batch the tests"

Incremental TDD is actually faster. You avoid over-engineering and let the design emerge.

## Integration with Claude Code

This skill works with:
- `/commit` - commits after each RED-GREEN-REFACTOR cycle
- `adr-review` skill - validates against ADRs during REFACTOR
- Test runners - verifies all tests after each change

## Philosophy

TDD is not about having tests. It's about using tests to discover the right design through tiny incremental steps.

The constraint of one failing test is what makes the process work.
