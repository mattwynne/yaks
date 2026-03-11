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
    ['cd /some/path && yx state "foo" wip', "cd then yx via &&"],
    ['cd /some/path && dev check', "cd then dev via &&"],
    ['cd foo; yx show bar', "cd then yx via ;"],
    // Read-only commands
    ["ls -la", "listing files"],
    ["grep -r foo src/", "grep recursively"],
    ["cat src/main.rs", "cat a file"],
    ["find . -name '*.rs'", "find files"],
    ["head -20 file.txt", "head of file"],
    ["tail -f log", "tail follow"],
    ["wc -l file", "word count"],
    ["grep -rn foo src/", "grep with line numbers"],
    ["tree src/", "tree directory"],
    ["stat file.txt", "stat file"],
    ["du -sh .", "disk usage"],
    ["echo 'hello'", "echo text"],
    ["which grep", "which command"],
    ["type ls", "type command"],
    ["realpath .", "realpath"],
    ["dirname /path/to/file", "dirname"],
    ["basename /path/to/file", "basename"],
    ["diff file1 file2", "diff files"],
    // Pipelines of read-only commands
    ["grep foo | wc -l", "pipeline of read-only"],
    ["find . -name '*.rs' | grep test", "find piped to grep"],
    ["cat file.txt | head -10", "cat piped to head"],
    // cd then read-only
    ["cd src && grep foo *.rs", "cd then read-only via &&"],
    ["cd src; ls -la", "cd then read-only via ;"],
    // git push (safe — doesn't modify the working tree)
    ["git push", "bare git push"],
    ["git push origin main", "git push with remote and branch"],
    ["git push --force-with-lease", "git push with flags"],
  ])("allows: %s (%s)", (cmd) => {
    expect(isAllowedBashCommand(cmd)).toBe(true);
  });

  it.each([
    ["git commit -m 'foo'", "git commit"],
    ["git checkout feature-branch", "git checkout"],
    ["git pull", "git pull modifies working tree"],
    ["git merge foo", "git merge modifies working tree"],
    ["git rebase main", "git rebase modifies working tree"],
    ["git stash", "git stash modifies working tree"],
    ['echo "hello" > file.txt', "echo redirect (write)"],
    ["cargo build", "cargo build"],
    ['sed -i "s/foo/bar/" file.txt', "sed in-place"],
    ["rm -rf src/", "rm command"],
    ["touch newfile.txt", "touch creates files"],
    ["mkdir newdir", "mkdir creates directories"],
    ["mv file1 file2", "mv moves files"],
    ["cp file1 file2", "cp copies files"],
    ["npm install", "npm install"],
    ["cargo test", "cargo test"],
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
