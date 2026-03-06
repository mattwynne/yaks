/**
 * Yak Tools Extension
 *
 * Registers tools for interacting with yaks:
 * - "show_yak": retrieve yak details via `yx show --format json`
 * - "list_yaks": list all yaks via `yx list --format json`
 */

import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";

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
}
