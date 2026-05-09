---
description: Run a three-model review with Gemini, Codex, and Claude, then synthesize the reports
---

Run a parallel three-model review for this request:

```
$@
```

Workflow:

1. Create a temporary review directory under the system temp directory, named with a `pi-review-` prefix. Keep the path for the final answer.
2. Use the `subagent` tool with `agentScope` set to `"both"` and `confirmProjectAgents` set to `false`.
3. Dispatch these three project-local agents in parallel:
   - `deep-review-gemini`
   - `deep-review-codex`
   - `deep-review-claude`
4. Give each subagent the same review request, plus:
   - the temporary review directory path;
   - its exact report output path:
     - `gemini.md`
     - `codex.md`
     - `claude.md`
   - an instruction to write its complete standalone report to that path;
   - an instruction not to edit repository files.
5. Wait for all three reports. If one model fails, continue with the reports that exist and call out the failure in the synthesis.
6. Read the report files from the temporary directory.
7. Synthesize the findings into a single report for Matt.

Subagent task template:

```
Review request:

<paste the user's request here>

Write a standalone review report to:

<temporary review directory>/<model-name>.md

Rules:

- Do not modify repository files.
- You may write only the report file named above.
- Use read-only exploration commands only.
- Focus on concrete findings with evidence.
- Include file paths and line numbers where applicable.
- Separate high-confidence findings from lower-confidence concerns.
- End with a short "Top recommendations" section.
```

Synthesis format:

```
# Review Synthesis

Temporary reports: <directory>

## Executive Summary

## Findings

Group findings by severity. For each finding, include:

- severity;
- concise title;
- evidence and file references;
- which model(s) raised it;
- your synthesis of whether it is valid;
- recommended action.

## Disagreements Or Single-Model Findings

## Gaps And Follow-Up Checks

## Source Reports

- Gemini: <path>
- Codex: <path>
- Claude: <path>
```

Keep the final synthesized report direct and decision-oriented. Do not paste the full three source reports unless the user asks for them.
