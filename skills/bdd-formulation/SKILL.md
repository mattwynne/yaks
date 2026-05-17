---
name: bdd-formulation
description: Use when writing or reviewing Gherkin scenarios, especially after discovering examples or edge cases that reveal a business rule
---

# BDD Formulation

Formulation is the craft of writing Gherkin that serves as
living documentation. Good scenarios are concrete examples
of business rules, not test scripts.

## Core Habit

When examples reveal a rule, make the rule visible in the
feature file: group related scenarios under a `Rule:` keyword
that states the rule they illustrate.

```gherkin
Rule: Manual blockers replace the existing manual blocker

  Scenario: Adding another manual blocker updates the reason
    Given I add the yak "deploy"
    And I add manual blocker to "deploy" with reason "waiting on vendor"
    When I add manual blocker to "deploy" with reason "waiting on review"
    Then the JSON yak "deploy" should have exactly one manual blocker with reason "waiting on review"
```

## BRIEF Check

From Seb Rose, scenarios should be:

- **B**usiness language: terms stakeholders understand.
- **R**eal data: vivid, concrete examples.
- **I**ntention revealing: say what matters, not how the UI works.
- **E**ssential: every line serves the rule.
- **F**ocused: one rule per scenario, short enough to read.

## Guidelines

- Use `Rule:` for a business/domain rule, not an implementation mechanism.
- Keep scenario names as concrete examples of the rule, not restatements of it.
- If a scenario discovers a new rule, add or split out a new `Rule:` section.
- If a scenario appears to illustrate multiple rules, split it.
- Prefer several small `Rule:` groups over one long pile of loosely related scenarios.
- Carry one consistent narrative through a feature where possible.
- Put cross-cutting assertions, such as output or logging, on existing scenarios rather than in separate rules.

## Review Questions

- What rule does this scenario prove?
- Does the Example name add information beyond the Rule?
- Is every line essential?
- Is the data concrete and business-readable?
- Would a feature with no `Rule:` sections be clearer if its rules were extracted?
