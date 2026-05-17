---
name: bdd-discovery
description: Use before writing Gherkin when exploring a story's behaviour, rules, examples, questions, scope, or readiness with stakeholders
---

# BDD Discovery

Discovery is the conversation before formulation. Use it to build
shared understanding of a story: the rules, examples, questions, and
scope boundaries that matter.

For writing or refining Gherkin, use `bdd-formulation`.

## Example Mapping

Map one story with the Three Amigos: product, development, and testing.
Keep it small, low-tech, and time-boxed.

Cards:

- **Story**: the work under discussion.
- **Rules**: business rules, constraints, policies, or acceptance criteria.
- **Examples**: concrete cases that illustrate one rule.
- **Questions**: unknowns, assumptions, missing decisions, or research.
- **New stories**: behaviour discovered but sliced out of scope.

Traditional colours: yellow story, blue rules, green examples, red questions.

## How to Run It

1. Start with the story and the rules people already know.
2. Ask for concrete examples for each rule.
3. Put each example under the rule it illustrates.
4. Capture uncertainty as questions; do not solve everything in the room.
5. Capture tangents or large discoveries as new stories.
6. Stop when the story is clear enough to pull, or the time-box expires.

## Discovery Habits

- Prefer conversation over documents.
- Use domain language, not implementation details.
- Talk about behaviour as if it could be handled manually.
- Keep examples rough but concrete: real names, amounts, states, dates.
- Treat red cards as progress: unknown unknowns became known unknowns.
- Let rules and examples reveal better story slices.

## Reading the Map

- Many red cards: too much uncertainty; research or invite the right person.
- Many blue cards: story may be too broad or complex.
- Many green cards under one rule: the rule may hide smaller rules.
- New story cards: useful scope control, not failure.

## Ready for Formulation?

Move to `bdd-formulation` when:

- The team agrees what problem the story solves.
- Key rules are visible.
- Risky or unclear rules have concrete examples.
- Open questions are captured and owned.
- Out-of-scope behaviour is sliced away.

## Source

- Cucumber: “Example Mapping” (cucumber.io/docs/bdd/example-mapping/)
