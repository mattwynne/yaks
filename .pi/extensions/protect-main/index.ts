import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { isToolCallEventType } from "@mariozechner/pi-coding-agent";
import { isMainRepo, decide } from "./rules.js";
import type { ToolType } from "./rules.js";
import { find, search } from "./tools.js";
import type { FindParams, SearchParams } from "./tools.js";

export default function (pi: ExtensionAPI) {
  // Register find tool
  pi.registerTool({
    name: "find",
    description: "Find files by name/path pattern. Respects .gitignore patterns (skips node_modules, .git, etc.)",
    parameters: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "Directory to search in (required)",
        },
        pattern: {
          type: "string",
          description: "Glob pattern to filter file names (e.g., '*.ts', 'README*')",
        },
        recursive: {
          type: "boolean",
          description: "Recurse into subdirectories (default: true)",
          default: true,
        },
      },
      required: ["path"],
    },
    handler: async (params: FindParams) => {
      return await find(params);
    },
  });

  // Register search tool
  pi.registerTool({
    name: "search",
    description: "Search for content within files. Returns matching lines with file paths and line numbers. Respects .gitignore patterns.",
    parameters: {
      type: "object",
      properties: {
        pattern: {
          type: "string",
          description: "String or regex pattern to search for in file contents (required)",
        },
        path: {
          type: "string",
          description: "Directory to search in (default: current directory)",
        },
        glob: {
          type: "string",
          description: "File type filter glob pattern (e.g., '*.feature', '*.{js,ts}')",
        },
      },
      required: ["pattern"],
    },
    handler: async (params: SearchParams) => {
      return await search(params);
    },
  });

  pi.on("tool_call", async (event, _ctx) => {
    if (!isMainRepo(process.cwd())) return;

    let tool: ToolType;
    let bashCommand: string | undefined;

    if (isToolCallEventType("read", event)) {
      tool = "read";
    } else if (isToolCallEventType("bash", event)) {
      tool = "bash";
      bashCommand = event.input.command;
    } else if (isToolCallEventType("write", event)) {
      tool = "write";
    } else if (isToolCallEventType("edit", event)) {
      tool = "edit";
    } else {
      tool = "other";
    }

    const decision = decide(tool, bashCommand);
    if (!decision.allowed) {
      return { block: true, reason: decision.reason! };
    }
  });
}
