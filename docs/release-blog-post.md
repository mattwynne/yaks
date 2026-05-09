# Introducing yaks — a discovery tree CLI for humans and AI

*You set out to do A, discover you need B first, which requires C. You've been yak shaving. The question is: how do you track it?*

I've spent the last three months building a tool to answer that question. It's called yaks — a CLI for managing discovery trees, designed to work for both humans and AI agents. This is the story of what I built, why I built it, and what I learned along the way.

---

## I've never read the code

Here's a thing about yaks that surprises people when I tell them: I've never written Rust. I've never read the code. Yet it's one of the codebases I'm happiest with — clean architecture, rigorous tests, a design that keeps holding up as I add new features.

How?

The short version is: I used AI to write all the code, and I used craft practices to keep the AI honest.

---

## The spectrum

Most developers using AI today are at what I think of as the REPL end of the spectrum. You type a prompt, the AI responds, you iterate. It's powerful — but you're in the loop every single turn.

At the other end is what Steve Yegge calls a Dark Factory: dozens of agents running in parallel, coordinated by an orchestrator, converging on a solution over hours. No human in the loop at all. To get there, you need a harness — a set of automated constraints that keep the AI accountable without you.

In between is where I spend most of my time: **sprints**. You plan together, leave the AI to implement for 20 minutes to an hour, then come back and review. Bigger chunks than REPL. Less infrastructure than a dark factory.

The question sprints raise is: what stops the AI going off the rails while you're away?

---

## The harness

The answer, for me, is a set of craft practices that I'd always valued but never been able to afford consistently — until AI made them cheap enough to use all the time.

**BDD scenarios** are the most important. A Cucumber scenario is a boundary object — a precise description of behaviour that both I and the AI can read and verify, without either of us having to look at the implementation. When I want the AI to implement something, I write a scenario first. When the AI drifts, the failing scenario tells me exactly what broke. Here's a real one, from the sync feature:

```gherkin
Rule: Concurrent changes by different users converge to the same state

  Example: Alice and bob both make changes and converge after syncing
    Given a git clone of origin called alice
    And a git clone of origin called bob
    And alice has a yak called "make the tea"
    And alice has synced yaks
    And bob has synced yaks
    And alice has set the state of "make the tea" to "wip"
    And bob has set the context of "make the tea" to "use the good teapot"
    And bob has synced yaks
    And alice has synced yaks
    When bob syncs yaks
    Then alice yak "make the tea" should have state "wip"
    And alice yak "make the tea" should have context "use the good teapot"
```

Distributed systems behaviour — CRDT-style merge semantics — in plain English. I wrote this. The AI implemented it. I never had to read the Rust.

**Architecture Decision Records** (ADRs) are the second layer. I review them periodically against the codebase, asking the AI: *read the ADRs, read the code, tell me where they diverge*. At one point, `main.rs` had ballooned to thousands of lines because it wasn't obvious to the model that hexagonal architecture applies to the UI layer too. The review caught it. I wrote ADR 0008 — "keep main.rs thin" — and the AI has followed it ever since.

**Mutation testing** validates the harness itself. A survived mutant means your tests don't actually check what you think they do. I'd never had time to set up mutation testing before. Now I run it on every diff. It costs almost nothing — and it's caught more than a few places where the AI had written plausible-looking tests that proved nothing.

The virtuous cycle: practices make the AI trustworthy, and the AI makes the practices affordable. I do more refactoring now than I ever did before.

---

## Why I built yaks

I didn't set out to build a task tracker. I set out to find a good way to track my own discovery trees — the branching, emergent structure of work that shows up when you're building with AI.

I tried Steve Yegge's Beads — a dependency-graph issue tracker designed for agents. I liked the idea of keeping the plan in git. But I quickly realised that plan changes and code changes are related but orthogonal. If you update your plan on a feature branch, nobody can see it until you merge. The plan needs to be visible to the whole team immediately, while the code stays isolated. Branches are the wrong model for shared working memory.

The existing tools — Jira, Notion, Linear — were built for a different era. They assume you know what you're building before you start. When you're moving fast and discovering as you go, they get in the way.

What I wanted was something simple: in git, but outside the branch model; designed for humans and AI equally; and small enough that I could understand the whole thing.

---

## Introducing yaks

Yaks stores everything in a hidden git ref — `refs/notes/yaks` — that sits outside your branch history. You sync it independently of your code. The plan travels with the repo but doesn't interfere with it.

The CLI is designed to work for humans (readable, ergonomic, tab-completed) and for AI agents (JSON output, fuzzy matching, different help text depending on who's asking). It's a discovery tree, not a backlog — you nest goals as you find them, move things around as you learn, mark them done as you go.

*[screenshot: yx ls showing the real project tree]*

It has states (`todo`, `wip`, `blocked`, `done`), context and custom fields, tags, sync, an event log, and tab completion. It's deliberately minimal — no epics, stories or tasks. Bring your own workflow.

*[screenshot or gif of yx sync]*

It's been my primary working memory for three months. It's also what Ralph — an autonomous Claude loop I demo'd at AONW in March — uses to plan and track its own work while refactoring code unsupervised.

---

## The shadow side

I'd be doing you a disservice if I didn't mention this: building with AI is genuinely addictive. Sending a prompt and waiting for a non-deterministic result feels like pulling a slot machine lever. Steve Yegge calls it the AI Vampire — the fatigue is real, even as output skyrockets.

My antidote: set your intentions before you start a session. Set your boundaries. Know what done looks like before you open a context window.

I use a yak.

---

## Try it

Yaks is open source. You can install it from GitHub and have it running in a few minutes.

→ [github.com/mattwynne/yaks](https://github.com/mattwynne/yaks)

I'd love to know what you think.
