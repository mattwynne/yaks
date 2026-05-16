import * as fs from "node:fs";
import * as path from "node:path";

export function isMainRepo(cwd: string): boolean {
  const gitPath = path.join(cwd, ".git");
  try {
    return fs.statSync(gitPath).isDirectory();
  } catch {
    return false;
  }
}

export type ToolType = "read" | "write" | "edit" | "bash" | "other";

export interface Decision {
  allowed: boolean;
  reason?: string;
}

const BLOCK_REASON =
  "🛑 Blocked: you're in the main repo, not a worktree. " +
  "Bash, write, and edit are blocked here. Use `read` plus the read-only `repo_ls`, `repo_find`, and `repo_rg` tools, " +
  "or start a worktree before making changes.";

function bashBlockReason(command?: string, logPath?: string): string {
  const commandText = command ? ` Attempted command: ${command}.` : "";
  const logText = logPath ? ` Logged to ${logPath}.` : "";
  return `${BLOCK_REASON}${commandText}${logText}`;
}

export function decide(tool: ToolType, bashCommand?: string, bashLogPath?: string): Decision {
  if (tool === "read" || tool === "other") {
    return { allowed: true };
  }

  if (tool === "bash") {
    return { allowed: false, reason: bashBlockReason(bashCommand, bashLogPath) };
  }

  return { allowed: false, reason: BLOCK_REASON };
}
