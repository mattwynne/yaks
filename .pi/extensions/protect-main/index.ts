import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { isToolCallEventType } from "@mariozechner/pi-coding-agent";
import { isMainRepo, decide } from "./rules.js";
import type { ToolType } from "./rules.js";

export default function (pi: ExtensionAPI) {
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
