import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { resolveWorktreePath } from "./index.js";

describe("resolveWorktreePath", () => {
  let originalCwd: string;
  let repoDir: string;

  beforeEach(() => {
    originalCwd = process.cwd();
    repoDir = fs.mkdtempSync(path.join(os.tmpdir(), "protect-main-worktree-"));
    fs.mkdirSync(path.join(repoDir, ".worktrees"));
    process.chdir(repoDir);
  });

  afterEach(() => {
    process.chdir(originalCwd);
    fs.rmSync(repoDir, { recursive: true, force: true });
  });

  it("resolves a git worktree child under .worktrees", () => {
    const worktree = path.join(repoDir, ".worktrees", "feature");
    fs.mkdirSync(worktree);
    fs.writeFileSync(path.join(worktree, ".git"), "gitdir: ../../.git/worktrees/feature\n");

    expect(resolveWorktreePath(".worktrees/feature")).toBe(path.join(process.cwd(), ".worktrees", "feature"));
  });

  it("rejects paths that escape the repository root", () => {
    expect(() => resolveWorktreePath("../outside")).toThrow(/escapes repository root/);
  });

  it("rejects .worktrees itself", () => {
    expect(() => resolveWorktreePath(".worktrees")).toThrow(/child path under \.worktrees\//);
  });

  it("rejects paths outside .worktrees", () => {
    const dir = path.join(repoDir, "other");
    fs.mkdirSync(dir);
    fs.writeFileSync(path.join(dir, ".git"), "gitdir: ../.git/worktrees/other\n");

    expect(() => resolveWorktreePath("other")).toThrow(/child path under \.worktrees\//);
  });

  it("rejects .worktrees children without a .git file", () => {
    fs.mkdirSync(path.join(repoDir, ".worktrees", "not-a-worktree"));

    expect(() => resolveWorktreePath(".worktrees/not-a-worktree")).toThrow(/missing \.git file/);
  });

  it("rejects .worktrees children with a .git directory", () => {
    const dir = path.join(repoDir, ".worktrees", "main-repo");
    fs.mkdirSync(path.join(dir, ".git"), { recursive: true });

    expect(() => resolveWorktreePath(".worktrees/main-repo")).toThrow(/\.git is not a file/);
  });
});
