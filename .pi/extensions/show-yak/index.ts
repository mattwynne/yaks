/**
 * Yak Tools Extension
 *
 * Registers tools for interacting with yaks:
 * - "show_yak": retrieve yak details via `yx show --format json`
 * - "list_yaks": list all yaks via `yx list --format json`
 * - "update_yak_context": update a yak's context via `yx context`
 */

import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";
import { writeFileSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export default function showYakExtension(pi: ExtensionAPI) {
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
}
