import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { isToolCallEventType } from "@mariozechner/pi-coding-agent";

export default function (pi: ExtensionAPI) {
  pi.on("tool_call", async (event, ctx) => {
    if (!isToolCallEventType("bash", event)) return;

    const cmd = event.input.command;

    // Check if the command includes a git commit (but not merge --ff-only or rebase)
    const isGitCommit = /\bgit\s+commit\b/.test(cmd);
    if (!isGitCommit) return;

    // Parse -C flag if present (handles quoted paths like git -C "/path with spaces" commit)
    const cDirMatch = cmd.match(/\bgit\s+(?:.*\s+)?-C\s+(?:"([^"]+)"|'([^']+)'|(\S+))/);
    const cDir = cDirMatch ? (cDirMatch[1] || cDirMatch[2] || cDirMatch[3]) : undefined;

    // Check current branch in the specified directory (or cwd if no -C flag)
    const result = await pi.exec("git", ["branch", "--show-current"], {
      cwd: cDir,
    });
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
