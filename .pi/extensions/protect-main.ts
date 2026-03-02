import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { isToolCallEventType } from "@mariozechner/pi-coding-agent";

export default function (pi: ExtensionAPI) {
  pi.on("tool_call", async (event, ctx) => {
    if (!isToolCallEventType("bash", event)) return;

    const cmd = event.input.command;

    // Check if the command includes a git commit (but not merge --ff-only or rebase)
    const isGitCommit = /\bgit\s+commit\b/.test(cmd);
    if (!isGitCommit) return;

    // Check current branch
    const result = await pi.exec("git", ["branch", "--show-current"], {});
    const branch = result.stdout.trim();

    if (branch === "main") {
      return {
        block: true,
        reason:
          "🛑 Blocked: you're trying to commit directly to main. " +
          "Create a feature branch and worktree first. " +
          "See the yak-worktree-workflow skill.",
      };
    }
  });
}
