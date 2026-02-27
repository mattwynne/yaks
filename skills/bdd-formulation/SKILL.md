---
name: bdd-formulation
description: Use when writing new Gherkin scenarios, refining existing feature files, or reviewing scenarios for clarity - applies BRIEF principles and BDD best practices to craft scenarios that serve as living documentation
---

# BDD Formulation

## Overview

Formulation is the craft of writing Gherkin that serves as
living documentation. Good scenarios are concrete examples
of business rules, not test scripts.

## When to Use

- Writing new Cucumber scenarios
- Reviewing or refining existing feature files
- A feature file feels bloated, unclear, or hard to maintain
- Scenario names restate the Rule or describe mechanisms

## The BRIEF Framework

From Seb Rose — six principles for scenario quality:

| Principle | Ask yourself | Anti-pattern |
|-----------|-------------|--------------|
| **B**usiness language | Would a stakeholder understand this? | "refs/notes/yaks", "fast-forward merge" |
| **R**eal data | Are the values vivid and concrete? | "test yak", "yak-a", "user1" |
| **I**ntention revealing | Does it say what, not how? | Click-by-click UI steps |
| **E**ssential | Does every line serve the rule? | Setup for unrelated concerns |
| **F**ocused | One rule per scenario? | Scenario fails from unrelated changes |
| **B**rief | Five lines or fewer? | Stakeholders skip long scenarios |

## Rules vs Examples

From Liz Keogh:

- **Rule** = acceptance criterion (abstract business rule)
- **Example** = scenario (concrete illustration with real data)

The Rule keyword states a crisp business rule. Examples
illustrate it with specific instances.

```gherkin
# BAD: Rule describes mechanism
Rule: Pulling yaks from origin

# GOOD: Rule states business rule
Rule: After syncing, all users have the same yaks
```

If the Example heading reads more like a rule than the
Rule does, promote it.

## Formulation Checklist

Review each scenario against these questions:

1. **Is the Rule a crisp business rule?**
   Not a mechanism, not vague. "Pushing to origin" is a
   mechanism. "Syncing stores yaks on the remote" is a rule.

2. **Does the Example name add information beyond the Rule?**
   Don't restate the rule. Name the specific instance.

3. **Is the data real and vivid, with a consistent narrative?**
   Pick one concrete story and carry it through the whole
   feature. Reuse the same entities across scenarios so
   the feature reads like a coherent narrative, not a
   collection of isolated test cases. Introduce new
   entities only when the rule demands it.

   ```gherkin
   # BAD: Each scenario invents unrelated data
   "test yak"... "deploy pipeline"... "security audit"...

   # GOOD: One story runs through the feature
   "make the tea"... "buy biscuits"... "wash the cups"...
   ```

4. **Is every line essential?**
   Remove setup that doesn't serve the rule. Collapse
   round-trips where possible (set state before syncing,
   not in a separate sync cycle).

5. **One rule per scenario?**
   If an example tests two rules, split it. If multiple
   examples under one rule test the same thing, merge them.

6. **Are cross-cutting concerns inline?**
   Output/logging assertions belong as additional Then
   steps on existing scenarios, not as separate rules.

7. **Does the docstring show exact output?**
   When asserting on output, use a docstring with precise
   expected text. Loose "should include" checks are vague.

## Review Process

When reviewing an existing feature file:

1. Read each Rule in turn
2. Ask: "What is the business rule here?"
3. Apply BRIEF to each Example
4. Look for duplicates across rules
5. Look for rules that are too broad — break them up
6. Ensure consistent narrative throughout
7. Check that setup is minimal

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Rule describes mechanism | State the business rule |
| Test labels as data | Use vivid, consistent narrative |
| Each scenario uses unrelated data | Carry one story through the whole feature |
| Example name restates rule | Name the specific instance |
| Multiple rules under one Rule | Break into focused rules |
| One broad rule, many examples | Does each example illustrate a different rule? |
| Separate rule for logging | Add assertions to existing scenarios |
| Unnecessary round-trips in setup | Collapse steps where possible |

## Sources

- Seb Rose: "Keep Your Scenarios BRIEF" (cucumber.io/blog)
- Liz Keogh: "Acceptance Criteria vs. Scenarios" (lizkeogh.com)
- Liz Keogh: "Step Away from the Tools" (lizkeogh.com)
- Liz Keogh: "It's about the examples you can't find" (lizkeogh.com)
