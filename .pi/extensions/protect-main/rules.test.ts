import { describe, it, expect, beforeAll, afterAll } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { decide, isMainRepo } from "./rules.js";

describe("isMainRepo", () => {
  let mainDir: string;
  let worktreeDir: string;
  let plainDir: string;

  beforeAll(() => {
    // Simulate a main repo: .git is a directory
    mainDir = fs.mkdtempSync(path.join(os.tmpdir(), "main-"));
    fs.mkdirSync(path.join(mainDir, ".git"));

    // Simulate a worktree: .git is a file
    worktreeDir = fs.mkdtempSync(path.join(os.tmpdir(), "worktree-"));
    fs.writeFileSync(
      path.join(worktreeDir, ".git"),
      "gitdir: /some/path/.git/worktrees/branch"
    );

    // Plain directory: no .git at all
    plainDir = fs.mkdtempSync(path.join(os.tmpdir(), "plain-"));
  });

  afterAll(() => {
    fs.rmSync(mainDir, { recursive: true });
    fs.rmSync(worktreeDir, { recursive: true });
    fs.rmSync(plainDir, { recursive: true });
  });

  it("returns true when .git is a directory", () => {
    expect(isMainRepo(mainDir)).toBe(true);
  });

  it("returns false when .git is a file (worktree)", () => {
    expect(isMainRepo(worktreeDir)).toBe(false);
  });

  it("returns false when there is no .git", () => {
    expect(isMainRepo(plainDir)).toBe(false);
  });
});

describe("decide", () => {
  it("allows read", () => {
    expect(decide("read")).toEqual({ allowed: true });
  });

  it("allows other tools (e.g. list_yaks, show_yak)", () => {
    expect(decide("other")).toEqual({ allowed: true });
  });

  it("blocks write", () => {
    const result = decide("write");
    expect(result.allowed).toBe(false);
    expect(result.reason).toMatch(/Bash, write, and edit are blocked/);
    expect(result.reason).toMatch(/repo_ls/);
  });

  it("blocks edit", () => {
    const result = decide("edit");
    expect(result.allowed).toBe(false);
    expect(result.reason).toMatch(/Bash, write, and edit are blocked/);
  });

  it.each([
    ["rg foo src/", "ripgrep"],
    ["yx show foo", "yx"],
    ["bin/dev check", "bin/dev"],
    ["git push", "git push"],
    ["git commit -m 'oops'", "git commit"],
    ["ls -la", "ls"],
  ])("blocks all bash commands including %s (%s)", (cmd) => {
    const result = decide("bash");
    expect(result.allowed).toBe(false);
    expect(result.reason).toMatch(/main repo/);

    const resultWithIgnoredCommand = decide("bash", cmd, ".pi/logs/protect-main-blocked-bash.jsonl");
    expect(resultWithIgnoredCommand.allowed).toBe(false);
    expect(resultWithIgnoredCommand.reason).toMatch(/repo_rg/);
    expect(resultWithIgnoredCommand.reason).toContain(cmd);
    expect(resultWithIgnoredCommand.reason).toContain(".pi/logs/protect-main-blocked-bash.jsonl");
  });

  it("blocks bash with no command", () => {
    const result = decide("bash");
    expect(result.allowed).toBe(false);
  });
});
