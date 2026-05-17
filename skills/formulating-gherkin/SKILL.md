---
name: formulating-gherkin
description: Use when writing or reviewing Gherkin features, especially after discovering examples or edge cases that reveal a new business rule
---

# Formulating Gherkin

When examples reveal a rule, make the rule visible in the
feature file.

## Core Habit

Group related scenarios under a `Rule:` keyword that states
the rule they illustrate.

```gherkin
Rule: Manual blockers replace the existing manual blocker

  Scenario: Adding another manual blocker updates the reason
    Given I add the yak "deploy"
    And I add manual blocker to "deploy" with reason "waiting on vendor"
    When I add manual blocker to "deploy" with reason "waiting on review"
    Then the JSON yak "deploy" should have exactly one manual blocker with reason "waiting on review"
```

## Guidelines

- Use `Rule:` for a business/domain rule, not an implementation mechanism.
- Put scenarios that demonstrate the same rule under the same `Rule:`.
- If a scenario discovers a new rule, add or split out a new `Rule:` section.
- Keep scenario names as concrete examples of the rule, not restatements of it.
- Prefer several small `Rule:` groups over one long pile of loosely related scenarios.
- When reviewing a feature, ask: “What rule does this scenario prove?” If the answer is unclear, reformulate, or discuss with your human
- If a scenario appears to illustate multiple rules, consider splitting it

## Smell

A feature with many scenarios and no `Rule:` sections is probably hiding the acceptance criteria. Extract the rules and group the scenarios around them.

For broader scenario formulation and BRIEF review, use `bdd-formulation`.
