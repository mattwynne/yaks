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

// Read-only commands that are safe to run in the main repo
const READ_ONLY_COMMANDS = new Set([
  "grep",
  "find",
  "ls",
  "cat",
  "head",
  "tail",
  "wc",
  "file",
  "tree",
  "stat",
  "du",
  "echo",
  "which",
  "type",
  "realpath",
  "dirname",
  "basename",
  "diff",
  "cd", // cd is safe as it doesn't modify files
]);

export function isAllowedBashCommand(cmd: string): boolean {
  const trimmed = cmd.trimStart();
  
  // Match dev/yx at start, after pipe, or after && or ;
  if (/(?:^|[|&;]\s*)(dev|bin\/dev|yx)\b/.test(trimmed)) {
    return true;
  }

  // Allow git push — it doesn't modify the working tree
  // But block all other git subcommands (commit, checkout, pull, merge, etc.)
  if (/(?:^|[|&;]\s*)git\s+push\b/.test(trimmed)) {
    return true;
  }

  // Block commands with output redirects (>, >>, 2>, &>, etc.)
  // These can write to files even if the base command is read-only
  if (/\s+[0-9]*>&?[0-9]*\s*[^|&;]/.test(trimmed) || /\s+[0-9]*>>?\s*[^|&;]/.test(trimmed)) {
    return false;
  }

  // Extract the first command word (handle cd && command patterns)
  // Split by && or ; to check each command in a chain
  const commandChain = trimmed.split(/\s*[;&]\s*/).filter(Boolean);
  
  for (const subcmd of commandChain) {
    const pipelineParts = subcmd.split(/\s*\|\s*/).filter(Boolean);
    
    // Check each command in the pipeline
    for (const part of pipelineParts) {
      const words = part.trim().split(/\s+/);
      const firstCommand = words[0];
      
      // Allow if it's a read-only command or one of our allowed commands
      if (!firstCommand) continue;
      
      const isReadOnly = READ_ONLY_COMMANDS.has(firstCommand);
      const isAllowedTool = /^(dev|bin\/dev|yx)$/.test(firstCommand);
      
      if (!isReadOnly && !isAllowedTool) {
        return false;
      }
    }
  }
  
  return true;
}

export type ToolType = "read" | "write" | "edit" | "bash" | "other";

export interface Decision {
  allowed: boolean;
  reason?: string;
}

const BLOCK_REASON =
  "🛑 Blocked: you're in the main repo, not a worktree. " +
  "Only `read`, `yx *`, `bin/dev *`, and read-only bash commands (grep, find, ls, cat, etc.) are allowed here. " +
  "To start work on a yak: `bin/dev start <yak-name>`, " +
  "then use a subagent with cwd set to the worktree path.";

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
