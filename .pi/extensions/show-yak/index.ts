/**
 * Yak Tools Extension
 *
 * Registers tools for interacting with yaks:
 * - "yx": run the `yx` CLI with structured arguments
 * - "show_yak": retrieve yak details via `yx show --format json`
 * - "list_yaks": list all yaks via `yx list --format json`
 * - "update_yak_context": update a yak's context via `yx context`
 * - "start_yak": create a worktree and start working on a yak
 * - "merge_yak": merge a yak branch back to main
 */

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { writeFileSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export default function showYakExtension(pi: ExtensionAPI) {
  pi.registerTool({
    name: "yx",
    label: "YX",
    description: "Run the `yx` CLI with structured arguments (no shell).",
    parameters: Type.Object({
      args: Type.Array(Type.String(), { description: "Arguments to pass to yx" }),
      timeoutMs: Type.Optional(Type.Number({ description: "Timeout in milliseconds (default: 30000)" })),
    }),

    async execute(toolCallId, params, signal) {
      const result = await pi.exec("yx", params.args, {
        signal,
        timeout: params.timeoutMs ?? 30000,
      });

      const output = (result.stderr ? `${result.stdout}\n${result.stderr}` : result.stdout).trim();

      if (result.code !== 0) {
        return {
          content: [{ type: "text", text: output }],
          isError: true,
        };
      }

      return {
        content: [{ type: "text", text: output }],
      };
    },
  });

  pi.registerTool({
    name: "list_yaks",
    label: "List Yaks",
    description:
      "List all yaks as a JSON tree via `yx list --format json`. " +
      "Optionally filter by state (only: 'done' or 'not-done') or tag.",
    parameters: Type.Object({
      only: Type.Optional(Type.String({ description: "Filter by state: 'done' or 'not-done'" })),
      tag: Type.Optional(Type.String({ description: "Filter by tag (e.g. 'ready')" })),
    }),

    async execute(toolCallId, params, signal) {
      const args = ["list", "--format", "json"];
      if (params.only) {
        args.push("--only", params.only);
      }
      if (params.tag) {
        args.push("--tag", params.tag);
      }
      const result = await pi.exec("yx", args, {
        signal,
        timeout: 10000,
      });

      if (result.code !== 0) {
        const error = (result.stderr || result.stdout).trim();
        return {
          content: [{ type: "text", text: `Error: ${error}` }],
          isError: true,
        };
      }

      return {
        content: [{ type: "text", text: result.stdout.trim() }],
      };
    },
  });

  pi.registerTool({
    name: "show_yak",
    label: "Show Yak",
    description:
      "Show details of a yak (name, state, tags, context, children) by running `yx show --format json`. " +
      "Pass the yak name as space-separated words, e.g. 'protect main from direct changes'.",
    parameters: Type.Object({
      name: Type.String({ description: "The yak name (space-separated words)" }),
    }),

    async execute(toolCallId, params, signal) {
      const result = await pi.exec("yx", ["show", "--format", "json", ...params.name.split(/\s+/)], {
        signal,
        timeout: 10000,
      });

      if (result.code !== 0) {
        const error = (result.stderr || result.stdout).trim();
        return {
          content: [{ type: "text", text: `Error: ${error}` }],
          isError: true,
        };
      }

      return {
        content: [{ type: "text", text: result.stdout.trim() }],
      };
    },
  });

  pi.registerTool({
    name: "update_yak_context",
    label: "Update Yak Context",
    description:
      "Update a yak's context markdown by piping content to `yx context <name>`. " +
      "Pass the yak name as space-separated words and the full markdown content.",
    parameters: Type.Object({
      name: Type.String({ description: "The yak name (space-separated words)" }),
      content: Type.String({ description: "The markdown content to set as context" }),
    }),

    async execute(toolCallId, params, signal) {
      const tmpFile = join(tmpdir(), `yak-ctx-${toolCallId}.md`);
      writeFileSync(tmpFile, params.content);
      try {
        const nameArgs = params.name.split(/\s+/).map((w: string) => `"${w}"`).join(" ");
        const result = await pi.exec("bash", ["-c", `cat "${tmpFile}" | yx context ${nameArgs}`], {
          signal,
          timeout: 10000,
        });

        if (result.code !== 0) {
          const error = (result.stderr || result.stdout).trim();
          return {
            content: [{ type: "text", text: `Error: ${error}` }],
            isError: true,
          };
        }

        return {
          content: [{ type: "text", text: `Context updated for '${params.name}'` }],
        };
      } finally {
        try { unlinkSync(tmpFile); } catch {}
      }
    },
  });

  pi.registerTool({
    name: "start_yak",
    label: "Start Yak",
    description:
      "Start working on a yak: marks it as wip, creates a git worktree, and returns the worktree path and branch name. " +
      "Pass the yak name as space-separated words, e.g. 'unify Yak type'.",
    parameters: Type.Object({
      name: Type.String({ description: "The yak name (space-separated words)" }),
    }),

    async execute(toolCallId, params, signal) {
      const result = await pi.exec("bin/dev", ["start", params.name], {
        signal,
        timeout: 30000,
      });

      const output = (result.stdout + "\n" + result.stderr).trim();

      if (result.code !== 0) {
        return {
          content: [{ type: "text", text: `Error starting yak:\n${output}` }],
          isError: true,
        };
      }

      // Parse worktree path and branch from output
      // Format: "✅ Worktree ready: <path> (branch: <branch>)"
      // Or if already exists: "Worktree already exists: <path>"
      let worktreePath = "";
      let branch = "";

      const readyMatch = output.match(/Worktree ready: (\S+) \(branch: (\S+)\)/);
      const existsMatch = output.match(/Worktree already exists: (\S+)/);

      if (readyMatch) {
        worktreePath = readyMatch[1];
        branch = readyMatch[2];
      } else if (existsMatch) {
        worktreePath = existsMatch[1];
        // Branch name is typically the last path segment
        branch = worktreePath.split("/").pop() || "";
      }

      const response: Record<string, string> = {
        worktreePath,
        branch,
        output,
      };

      return {
        content: [{ type: "text", text: JSON.stringify(response, null, 2) }],
      };
    },
  });

  pi.registerTool({
    name: "merge_yak",
    label: "Merge Yak",
    description:
      "Merge a yak branch back to main. Runs checks, rebases, fast-forward merges, and cleans up the worktree. " +
      "Pass the branch name (typically the yak ID, e.g. 'unify-yak-type-lhf6').",
    parameters: Type.Object({
      branch: Type.String({ description: "The branch name to merge (typically the yak ID)" }),
    }),

    async execute(toolCallId, params, signal) {
      const result = await pi.exec("bin/dev", ["merge", params.branch], {
        signal,
        timeout: 300000,
      });

      const output = (result.stdout + "\n" + result.stderr).trim();

      if (result.code !== 0) {
        return {
          content: [{ type: "text", text: `Merge failed:\n${output}` }],
          isError: true,
        };
      }

      return {
        content: [{ type: "text", text: output }],
      };
    },
  });
}
