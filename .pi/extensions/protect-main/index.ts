import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { isToolCallEventType } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import * as path from "node:path";
import * as fs from "node:fs";
import { isMainRepo, decide } from "./rules.js";
import type { ToolType } from "./rules.js";
import {
  appendBlockedBashLog,
  blockedBashLogPath,
  readRecentBlockedBashLog,
  BLOCKED_BASH_LOG_RELATIVE_PATH,
} from "./log.js";

export function resolveRepoPath(requestedPath = "."): string {
  const root = process.cwd();
  const resolved = path.resolve(root, requestedPath);
  const relative = path.relative(root, resolved);

  if (relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative))) {
    return resolved;
  }

  throw new Error(`Path escapes repository root: ${requestedPath}`);
}

export function resolveWorktreePath(requestedPath: string): string {
  const root = process.cwd();
  const resolved = resolveRepoPath(requestedPath);
  const relative = path.relative(root, resolved);

  if (relative === ".worktrees" || !relative.startsWith(`.worktrees${path.sep}`)) {
    throw new Error(`Path must name a child path under .worktrees/: ${requestedPath}`);
  }

  const gitFile = path.join(resolved, ".git");
  let stats: fs.Stats;
  try {
    stats = fs.statSync(gitFile);
  } catch {
    throw new Error(`Path is not a git worktree (missing .git file): ${requestedPath}`);
  }

  if (!stats.isFile()) {
    throw new Error(`Path is not a git worktree (.git is not a file): ${requestedPath}`);
  }

  return resolved;
}

export function limitLines(text: string, limit?: number): string {
  if (limit === undefined) return text;
  return text.split(/\r?\n/).slice(0, Math.max(0, limit)).join("\n");
}

export function clampMaxCount(maxCount: number | undefined, defaultValue: number): number {
  return Math.min(Math.max(0, maxCount ?? defaultValue), 100);
}

function relativeRepoPath(absolutePath: string): string {
  const relative = path.relative(process.cwd(), absolutePath);
  return relative === "" ? "." : relative;
}

async function runGit(pi: ExtensionAPI, cwd: string, args: string[], signal: AbortSignal) {
  const result = await pi.exec("git", ["-C", cwd, ...args], { signal, timeout: 10000 });
  if (result.code !== 0) {
    throw new Error((result.stderr || result.stdout).trim() || `git ${args.join(" ")} failed`);
  }
  return result.stdout.trim();
}

function errorResult(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return {
    content: [{ type: "text" as const, text: `Error: ${message}` }],
    isError: true,
  };
}

function commandError(command: string, stdout: string, stderr: string) {
  const output = (stderr || stdout).trim();
  return {
    content: [{ type: "text" as const, text: `Error running ${command}: ${output}` }],
    isError: true,
  };
}

function bashCommandError(exitCode: number, stdout: string, stderr: string) {
  const output = [stdout, stderr].filter(Boolean).join("\n").trim();
  const text = output ? `Command exited with code ${exitCode}:\n${output}` : `Command exited with code ${exitCode}`;
  return {
    content: [{ type: "text" as const, text }],
    isError: true,
  };
}

export default function (pi: ExtensionAPI) {
  pi.registerTool({
    name: "repo_info",
    label: "Repo Info",
    description: "Return JSON describing the current repository and git state.",
    parameters: Type.Object({}),

    async execute(_toolCallId, _params, signal) {
      try {
        const cwd = process.cwd();
        const [currentBranch, gitTopLevel] = await Promise.all([
          runGit(pi, cwd, ["branch", "--show-current"], signal),
          runGit(pi, cwd, ["rev-parse", "--show-toplevel"], signal),
        ]);

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              cwd,
              repoRoot: cwd,
              currentBranch,
              gitTopLevel,
              isMainRepo: isMainRepo(cwd),
            }, null, 2),
          }],
        };
      } catch (error) {
        return errorResult(error);
      }
    },
  });

  pi.registerTool({
    name: "repo_git_status",
    label: "Repo Git Status",
    description: "Run git status --short --branch under a path inside the repository.",
    parameters: Type.Object({
      path: Type.Optional(Type.String({ description: "Path under the repository root (default: .)" })),
    }),

    async execute(_toolCallId, params, signal) {
      try {
        const target = resolveRepoPath(params.path);
        const output = await runGit(pi, target, ["status", "--short", "--branch"], signal);
        return { content: [{ type: "text" as const, text: output }] };
      } catch (error) {
        return errorResult(error);
      }
    },
  });

  pi.registerTool({
    name: "repo_git_log",
    label: "Repo Git Log",
    description: "Run git log --oneline under a path inside the repository.",
    parameters: Type.Object({
      path: Type.Optional(Type.String({ description: "Path under the repository root (default: .)" })),
      maxCount: Type.Optional(Type.Integer({ minimum: 0, description: "Maximum commits to return (default: 5, capped at 100)" })),
      decorate: Type.Optional(Type.Boolean({ description: "Include --decorate" })),
      range: Type.Optional(Type.String({ description: "Optional revision range, e.g. main..HEAD" })),
    }),

    async execute(_toolCallId, params, signal) {
      try {
        const target = resolveRepoPath(params.path);
        const args = ["log", "--oneline"];
        if (params.decorate) args.push("--decorate");
        args.push("--max-count", String(clampMaxCount(params.maxCount, 5)));
        if (params.range) args.push(params.range);
        const output = await runGit(pi, target, args, signal);
        return { content: [{ type: "text" as const, text: output }] };
      } catch (error) {
        return errorResult(error);
      }
    },
  });

  pi.registerTool({
    name: "worktree_statuses",
    label: "Worktree Statuses",
    description: "Return JSON status summaries for git worktrees under .worktrees/.",
    parameters: Type.Object({
      paths: Type.Optional(Type.Array(Type.String({ description: "Path to a git worktree under .worktrees/" }))),
      maxCommits: Type.Optional(Type.Integer({ minimum: 0, description: "Maximum recent commits per worktree (default: 5, capped at 100)" })),
    }),

    async execute(_toolCallId, params, signal) {
      const root = process.cwd();
      const requestedPaths = params.paths ?? (() => {
        const worktreesDir = path.join(root, ".worktrees");
        try {
          return fs.readdirSync(worktreesDir, { withFileTypes: true })
            .filter((entry) => entry.isDirectory() && fs.existsSync(path.join(worktreesDir, entry.name, ".git")))
            .map((entry) => path.join(".worktrees", entry.name));
        } catch {
          return [];
        }
      })();
      const maxCommits = clampMaxCount(params.maxCommits, 5);

      const statuses = [];
      for (const requestedPath of requestedPaths) {
        try {
          const worktreePath = resolveWorktreePath(requestedPath);
          const [branch, statusShortBranch, aheadBehindMain, recentCommits] = await Promise.all([
            runGit(pi, worktreePath, ["branch", "--show-current"], signal),
            runGit(pi, worktreePath, ["status", "--short", "--branch"], signal),
            runGit(pi, worktreePath, ["rev-list", "--left-right", "--count", "main...HEAD"], signal),
            runGit(pi, worktreePath, ["log", "--oneline", "main..HEAD", "--max-count", String(maxCommits)], signal),
          ]);
          statuses.push({
            path: relativeRepoPath(worktreePath),
            branch,
            statusShortBranch,
            aheadBehindMain,
            recentCommits,
          });
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          statuses.push({ path: requestedPath, error: message });
        }
      }

      return { content: [{ type: "text" as const, text: JSON.stringify(statuses, null, 2) }] };
    },
  });

  pi.registerTool({
    name: "repo_blocked_bash_log",
    label: "Repo Blocked Bash Log",
    description:
      "Read-only view of recent protect-main blocked bash attempts logged under .pi/logs/.",
    parameters: Type.Object({
      limit: Type.Optional(Type.Integer({ minimum: 0, description: "Maximum recent entries to return (default: 20)" })),
    }),

    async execute(_toolCallId, params) {
      try {
        const entries = readRecentBlockedBashLog(blockedBashLogPath(), params.limit ?? 20);
        if (entries.length === 0) {
          return {
            content: [
              {
                type: "text",
                text: `No blocked bash attempts logged yet at ${BLOCKED_BASH_LOG_RELATIVE_PATH}.`,
              },
            ],
          };
        }

        return { content: [{ type: "text", text: JSON.stringify(entries, null, 2) }] };
      } catch (error) {
        return errorResult(error);
      }
    },
  });

  pi.registerTool({
    name: "repo_ls",
    label: "Repo LS",
    description:
      "Read-only ls inside the main repository. Paths are resolved under the repository root.",
    parameters: Type.Object({
      path: Type.Optional(Type.String({ description: "Path under the repository root (default: .)" })),
      all: Type.Optional(Type.Boolean({ description: "Include dotfiles (-a)" })),
      long: Type.Optional(Type.Boolean({ description: "Use long listing format (-l)" })),
      limit: Type.Optional(Type.Integer({ minimum: 0, description: "Maximum output lines" })),
    }),

    async execute(_toolCallId, params, signal) {
      let target: string;
      try {
        target = resolveRepoPath(params.path);
      } catch (error) {
        return errorResult(error);
      }

      const args: string[] = [];
      if (params.all && params.long) args.push("-la");
      else if (params.all) args.push("-a");
      else if (params.long) args.push("-l");
      args.push(target);

      const result = await pi.exec("ls", args, { signal, timeout: 10000 });
      if (result.code !== 0) return commandError("ls", result.stdout, result.stderr);

      return { content: [{ type: "text", text: limitLines(result.stdout.trim(), params.limit) }] };
    },
  });

  pi.registerTool({
    name: "repo_find",
    label: "Repo Find",
    description:
      "Read-only find inside the main repository. Paths are resolved under the repository root.",
    parameters: Type.Object({
      path: Type.Optional(Type.String({ description: "Path under the repository root (default: .)" })),
      name: Type.Optional(Type.String({ description: "Name pattern passed to find -name" })),
      type: Type.Optional(
        Type.Union([
          Type.Literal("file"),
          Type.Literal("dir"),
          Type.Literal("any"),
        ], { description: "Entry type to match (default: any)" })
      ),
      maxDepth: Type.Optional(Type.Integer({ minimum: 0, description: "Maximum search depth" })),
      limit: Type.Optional(Type.Integer({ minimum: 0, description: "Maximum output lines" })),
    }),

    async execute(_toolCallId, params, signal) {
      let target: string;
      try {
        target = resolveRepoPath(params.path);
      } catch (error) {
        return errorResult(error);
      }

      const args = [target];
      if (params.maxDepth !== undefined) args.push("-maxdepth", String(params.maxDepth));
      if (params.type === "file") args.push("-type", "f");
      if (params.type === "dir") args.push("-type", "d");
      if (params.name) args.push("-name", params.name);

      const result = await pi.exec("find", args, { signal, timeout: 10000 });
      if (result.code !== 0) return commandError("find", result.stdout, result.stderr);

      return { content: [{ type: "text", text: limitLines(result.stdout.trim(), params.limit) }] };
    },
  });

  pi.registerTool({
    name: "repo_rg",
    label: "Repo RG",
    description:
      "Read-only ripgrep inside the main repository. Paths are resolved under the repository root.",
    parameters: Type.Object({
      pattern: Type.String({ description: "Search pattern" }),
      paths: Type.Optional(Type.Array(Type.String({ description: "Path under the repository root" }))),
      glob: Type.Optional(Type.Array(Type.String({ description: "Glob passed as --glob" }))),
      ignoreCase: Type.Optional(Type.Boolean({ description: "Case-insensitive search (-i)" })),
      hidden: Type.Optional(Type.Boolean({ description: "Search hidden files and directories (--hidden)" })),
      limit: Type.Optional(Type.Integer({ minimum: 0, description: "Maximum output lines" })),
    }),

    async execute(_toolCallId, params, signal) {
      let targets: string[];
      try {
        targets = (params.paths && params.paths.length > 0 ? params.paths : ["."]).map((p: string) =>
          resolveRepoPath(p)
        );
      } catch (error) {
        return errorResult(error);
      }

      const args = ["--line-number", "--column", "--no-heading", "--color", "never"];
      if (params.ignoreCase) args.push("--ignore-case");
      if (params.hidden) args.push("--hidden");
      for (const glob of params.glob ?? []) args.push("--glob", glob);
      args.push(params.pattern, ...targets);

      const result = await pi.exec("rg", args, { signal, timeout: 10000 });
      if (result.code !== 0 && result.code !== 1) {
        return commandError("rg", result.stdout, result.stderr);
      }

      return { content: [{ type: "text", text: limitLines(result.stdout.trim(), params.limit) }] };
    },
  });

  pi.registerTool({
    name: "worktree_bash",
    label: "Worktree Bash",
    description:
      "Execute a bash command in a selected git worktree under .worktrees/. Bash runs in the selected worktree, not main. Returns stdout and stderr trimmed.",
    parameters: Type.Object({
      path: Type.String({ description: "Path to a git worktree under .worktrees/" }),
      command: Type.String({ description: "Bash command to execute in the selected worktree" }),
      timeoutMs: Type.Optional(Type.Integer({ minimum: 0, description: "Timeout in milliseconds (default: 30000)" })),
    }),

    async execute(_toolCallId, params, signal) {
      let worktreePath: string;
      try {
        worktreePath = resolveWorktreePath(params.path);
      } catch (error) {
        return errorResult(error);
      }

      try {
        const result = await pi.exec("bash", ["-lc", params.command], {
          cwd: worktreePath,
          signal,
          timeout: params.timeoutMs ?? 30000,
        });
        const output = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
        if (result.code !== 0) return bashCommandError(result.code, result.stdout, result.stderr);

        return { content: [{ type: "text" as const, text: output }] };
      } catch (error) {
        return errorResult(error);
      }
    },
  });

  pi.on("tool_call", async (event, _ctx) => {
    if (!isMainRepo(process.cwd())) return;

    let tool: ToolType;

    if (isToolCallEventType("read", event)) {
      tool = "read";
    } else if (isToolCallEventType("bash", event)) {
      tool = "bash";
    } else if (isToolCallEventType("write", event)) {
      tool = "write";
    } else if (isToolCallEventType("edit", event)) {
      tool = "edit";
    } else {
      tool = "other";
    }

    const bashCommand = tool === "bash" && isToolCallEventType("bash", event) ? event.input.command : undefined;
    const bashLogPath = tool === "bash" ? BLOCKED_BASH_LOG_RELATIVE_PATH : undefined;
    const decision = decide(tool, bashCommand, bashLogPath);
    if (!decision.allowed) {
      if (tool === "bash") {
        appendBlockedBashLog({
          timestamp: new Date().toISOString(),
          cwd: process.cwd(),
          command: bashCommand ?? "",
        });
      }

      return { block: true, reason: decision.reason! };
    }
  });
}
