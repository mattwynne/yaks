/**
 * Show Yak Tool Extension
 *
 * Registers a "show_yak" tool that lets the LLM retrieve yak details
 * via `yx show --format json`.
 */

import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";

export default function showYakExtension(pi: ExtensionAPI) {
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
