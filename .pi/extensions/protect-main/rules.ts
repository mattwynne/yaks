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

export function isAllowedBashCommand(cmd: string): boolean {
  const trimmed = cmd.trimStart();
  // Match dev/yx at start, after pipe, or after && or ;
  return /(?:^|[|&;]\s*)(dev|bin\/dev|yx)\b/.test(trimmed);
}

export type ToolType = "read" | "write" | "edit" | "bash" | "other";

export interface Decision {
  allowed: boolean;
  reason?: string;
}

const BLOCK_REASON =
  "🛑 Blocked: you're in the main repo, not a worktree. " +
  "Only `read`, `dev *`, and `yx *` commands are allowed here. " +
  "Use `dev` commands to create a worktree first.";

export function decide(tool: ToolType, bashCommand?: string): Decision {
  if (tool === "read" || tool === "other") {
    return { allowed: true };
  }

  if (tool === "bash") {
    if (bashCommand && isAllowedBashCommand(bashCommand)) {
      return { allowed: true };
    }
    return { allowed: false, reason: BLOCK_REASON };
  }

  // write and edit
  return { allowed: false, reason: BLOCK_REASON };
}
