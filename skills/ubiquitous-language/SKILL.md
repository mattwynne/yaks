---
name: ubiquitous-language
description: Use when evaluating the ubiquitous language in a codebase - produces a glossary of domain terms with references and commentary on inconsistencies, awkward names, or overlapping concepts
---

# Ubiquitous Language Review

## Overview

The ubiquitous language is the shared vocabulary a team uses
for domain concepts — in conversation, code, tests, and docs.
When the language is consistent, the code communicates clearly.
When it drifts, misunderstandings hide in plain sight.

This skill produces a `docs/terms.md` glossary and a
commentary on the health of the language.

## When to Use

- Onboarding to a codebase — to understand the domain
- After significant refactoring or feature work
- When naming debates keep recurring
- When new team members are confused by terms
- Periodically, as a hygiene check

## Process

### 1. Harvest Terms

Scan the codebase for domain terms. Focus on these layers,
in order of authority:

| Layer | Where to look | Why |
|-------|--------------|-----|
| **Domain model** | Domain types, enums, enum variants, struct fields, constants, naming conventions, validation rules | The core vocabulary — highest authority |
| **Events** | Event types and their payloads | Capture what happened in domain language |
| **Commands / Use cases** | Application layer, command handler | The verbs — what users can do |
| **CLI surface** | CLI commands, flags, help text | How users encounter the language |
| **Feature files** | See detailed guidance below | The richest source of natural-language domain terms |
| **Other tests** | Unit test names, integration test names | How developers talk about behaviour |
| **Documentation** | README, ADRs, design docs | How the team explains the system |

#### Reading Feature Files

Feature files deserve special attention. They are
scenarios written in natural language — the closest thing
to how the team *talks* about the domain. Read them
carefully, extracting terms from:

- **Rule names** — these state business rules and often
  name domain distinctions that have no type in the code.
  A rule like "reserved fields cannot be overwritten"
  reveals the concept "reserved field" even though the code
  only has constants and a validation function.
- **Scenario names** — name specific instances and may
  reveal subcategories of a concept.
- **Given/When/Then step phrasing** — the steps *are* the
  ubiquitous language in sentence form. Terms that appear
  in steps but have no type in the code are candidates for
  missing abstractions.
- **Feature-level descriptions** — often name high-level
  concepts or provide context for why a group of rules
  exists.

Also ask: "For each noun in the glossary, are there named
subcategories?" Many domain concepts have subtypes that
aren't in the type system but live in conventions,
validation rules, or feature file Rules. "Reserved field"
vs "custom field" is a classification *within* the Field
concept. These boundaries matter.

Look for "negative space" terms — concepts the team uses
in conversation that have no name in the code. If a
feature file says "reserved fields" but the code has no
`ReservedField` type or even a comment using that phrase,
that's a gap worth flagging.

#### Judging Feature File Quality

Feature files should use **declarative** style — they
describe *what* the system does using domain language, not
*how* the user interacts with the UI or implementation.

**Declarative** (good): Steps express business intent.

```gherkin
Given a yak "make the tea" with state "done"
When I prune
Then "make the tea" should be removed
```

**Imperative** (bad): Steps spell out mechanics.

```gherkin
Given I run "yx add make the tea"
And I run "yx state make the tea done"
When I run "yx prune"
Then I run "yx ls" and the output does not contain
  "make the tea"
```

Signs of imperative Gherkin:

| Smell | What it looks like | Why it's bad |
|-------|--------------------|-------------|
| **Shell commands in steps** | `When I run "yx add ..."` | Steps are coupled to CLI implementation, not domain |
| **Output parsing** | `Then the output contains "..."` | Tests the rendering, not the behaviour |
| **Multi-step mechanics** | Many steps to set up one domain state | Business rule is buried in noise |
| **UI/CLI navigation** | "I type", "I press", "I see" | Not domain language |
| **No domain nouns** | Steps use generic terms like "output", "command", "result" | The ubiquitous language is absent |

When reviewing feature files for this glossary, flag:

- Steps that use implementation language instead of domain
  language — these are missed opportunities to exercise
  the ubiquitous language.
- Rules or scenarios that reveal domain concepts not
  reflected in the code's type system.
- Steps that could name a domain concept but instead
  describe a mechanism.

The test is: **"Would this wording need to change if the
implementation changed?"** If yes, the step is imperative
and the domain language is being bypassed.

Separate nouns and verbs. Domain nouns are the things the
system talks about. Domain verbs are the operations
performed on them. This separation matters because nouns
tend to have multiple representations (structs, views,
snapshots) while verbs tend to have multiple expressions
(CLI commands, use cases, events).

For each term, record:
- **Term**: The canonical name
- **Definition**: What it means in this domain (one sentence)
- **References**: Where it appears (files, with line numbers
  for definitions; just filenames for usage)
- **Aliases**: Any synonyms or variations found

### 2. Identify Issues

Look for these specific problems:

#### Synonyms
Different words for the same concept. Example: "task" in
docs but "yak" in code. Pick one. Synonyms cause people to
wonder whether the two words mean subtly different things.

#### Homonyms
Same word meaning different things in different contexts.
Example: "state" meaning both "yak lifecycle state" and
"application state". Flag these even if the context usually
disambiguates — they trip up newcomers.

#### Technical Leak
Implementation terms in the domain layer. The domain model
should speak the language of the problem, not the solution.
Examples: "aggregate" in a CLI message, "projection" in a
feature file, "store" where "repository" or a domain term
would be clearer.

#### Awkward Names
Terms that don't convey their meaning without explanation.
A name is awkward if you'd need a comment or a conversation
to understand it. Abbreviations, inside jokes, and legacy
names that no longer match the concept all qualify.

#### Missing Terms
Places where the code uses generic language ("item", "thing",
"data", "info") instead of a domain-specific term. Also
look for concepts that exist in conversation but have no
name in the code.

#### Inconsistent Granularity
Some terms are very specific while related terms are vague.
Example: detailed lifecycle states (`todo`, `wip`, `done`)
alongside a vague `update` event that covers many things.

#### Imperative Steps in Feature Files
Gherkin steps that describe *how* (shell commands, output
parsing, UI mechanics) instead of *what* (domain operations
and outcomes). Imperative steps bypass the ubiquitous
language — they test the implementation without exercising
the vocabulary. Flag specific steps and suggest declarative
rewrites using domain terms.

### 3. Inventory Every Name Honestly

For each domain concept, list **every type, struct, enum,
or alias** in the code that represents it. Do not gloss
over the fact that one concept may have multiple
representations. Name each one, say where it lives, explain
why it exists, and note what it relates to.

For example, if the concept "Yak" appears as `YakView`,
`YakEntry`, `YakSnapshot`, `YakTreeNode`, and `YakChildView`,
list all five. Explain: `YakView` is the read-model DTO,
`YakEntry` is the aggregate's internal state, `YakSnapshot`
is used for compaction, etc. This is the inventory — no
smoothing over, no "the code calls this a Yak".

Ask for each representation:
- Why does this one exist separately?
- What would break if it were merged with another?
- Does its name communicate its role clearly?
- Could a newcomer tell it apart from the others?

### 4. Write the Glossary

Create or update `docs/terms.md` with this structure:

```markdown
# Ubiquitous Language — Glossary

> This glossary defines the shared vocabulary for this
> project. When naming things in code, tests, CLI, or
> docs, use these terms consistently.
>
> Last reviewed: YYYY-MM-DD

## Nouns

### Term Name

Definition in one sentence.

**Representations in code:**

| Type | File | Role |
|------|------|------|
| `TypeName` | `path/to/file.rs:42` | Why this one exists |
| `OtherType` | `path/to/other.rs:10` | Why this one exists |

- **Used in**: `features/foo.feature`, `docs/bar.md`
- **Related to**: Other Term, Another Term
- **Aliases**: (list any synonyms — flag if problematic)

## Verbs

### Term Name

Definition in one sentence.

| Expression | Where | Form |
|------------|-------|------|
| `yx command` | CLI | User-facing command |
| `UseCaseName` | `src/application/file.rs:1` | Use case |
| `EventName` | `src/domain/event.rs:14` | Domain event |

- **Used in**: `features/foo.feature`
- **Related to**: Other Term

## Commentary

### Strengths
What the codebase does well with its language.

### Issues Found

#### Issue Title
- **Type**: Synonym / Homonym / Technical Leak / Awkward
  Name / Missing Term / Inconsistent Granularity
- **Where**: files and lines
- **Problem**: what's wrong
- **Suggestion**: how to fix it
```

### 5. Review Criteria

A healthy ubiquitous language has these properties:

| Property | Check |
|----------|-------|
| **One term per concept** | No synonyms |
| **One concept per term** | No homonyms |
| **Domain speaks for itself** | No technical leak |
| **Self-documenting** | No awkward names |
| **Complete** | No missing terms |
| **Consistent depth** | No granularity gaps |
| **Aligned across layers** | Same words in code, CLI, tests, docs |
| **Declarative tests** | Feature files use domain language, not implementation mechanics |

## What This Skill Does NOT Do

- It does not rename anything. It produces a glossary and
  commentary. Renaming is a separate decision.
- It does not judge architectural choices. "Event sourcing"
  is a valid implementation term in infrastructure — the
  issue is only when it leaks into domain language.
- It does not require perfection. Every codebase has some
  inconsistency. The goal is awareness, not zero issues.

## Sources

- Eric Evans: "Domain-Driven Design" (2003), Chapter 2
- Vaughn Vernon: "Implementing Domain-Driven Design" (2013)
- Alberto Brandolini: "Introducing EventStorming" — the
  shared language emerges from collaborative modelling
- "Declarative vs Imperative Gherkin Scenarios"
  (itsadeliverything.com) — why steps should express
  business intent, not implementation mechanics
- Cucumber docs: "Writing Better Gherkin"
  (cucumber.io/docs/bdd/better-gherkin/) — the litmus
  test: "will this wording need to change if the
  implementation does?"
