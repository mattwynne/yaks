import { describe, it, expect, beforeAll, afterAll } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { decide, isMainRepo, isAllowedBashCommand } from "./rules.js";

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

describe("isAllowedBashCommand", () => {
  it.each([
    ["dev check", "dev with subcommand"],
    ["dev", "bare dev"],
    ["bin/dev check", "bin/dev with subcommand"],
    ["bin/dev", "bare bin/dev"],
    ["yx show foo", "yx with subcommand"],
    ["yx", "bare yx"],
    ['echo "context" | yx context "foo"', "piping into yx"],
    ["yx ls | grep foo", "piping from yx"],
    ['cat <<EOF | yx context "foo"\nstuff\nEOF', "heredoc into yx"],
  ])("allows: %s (%s)", (cmd) => {
    expect(isAllowedBashCommand(cmd)).toBe(true);
  });

  it.each([
    ["ls -la", "listing files"],
    ["git commit -m 'foo'", "git commit"],
    ["git checkout feature-branch", "git checkout"],
    ['echo "hello" > file.txt', "echo redirect"],
    ["cargo build", "cargo build"],
    ["grep -r foo src/", "grep"],
    ["cat src/main.rs", "cat"],
    ['sed -i "s/foo/bar/" file.txt', "sed in-place"],
    ["rm -rf src/", "rm"],
  ])("blocks: %s (%s)", (cmd) => {
    expect(isAllowedBashCommand(cmd)).toBe(false);
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
    expect(result.reason).toMatch(/main repo/);
  });

  it("blocks edit", () => {
    const result = decide("edit");
    expect(result.allowed).toBe(false);
    expect(result.reason).toMatch(/main repo/);
  });

  it("allows dev bash commands", () => {
    expect(decide("bash", "dev check")).toEqual({ allowed: true });
  });

  it("allows yx bash commands", () => {
    expect(decide("bash", "yx show foo")).toEqual({ allowed: true });
  });

  it("blocks arbitrary bash commands", () => {
    const result = decide("bash", "git commit -m 'oops'");
    expect(result.allowed).toBe(false);
    expect(result.reason).toMatch(/main repo/);
  });

  it("blocks bash with no command", () => {
    const result = decide("bash");
    expect(result.allowed).toBe(false);
  });
});
