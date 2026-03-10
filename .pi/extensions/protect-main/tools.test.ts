import { describe, it, expect, beforeAll, afterAll } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { find, search } from "./tools.js";

describe("find", () => {
  let testDir: string;

  beforeAll(() => {
    // Create a test directory structure
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), "find-test-"));

    // Create files and directories
    fs.writeFileSync(path.join(testDir, "file1.txt"), "content1");
    fs.writeFileSync(path.join(testDir, "file2.js"), "content2");
    fs.writeFileSync(path.join(testDir, "README.md"), "readme");

    fs.mkdirSync(path.join(testDir, "src"));
    fs.writeFileSync(path.join(testDir, "src", "index.ts"), "code");
    fs.writeFileSync(path.join(testDir, "src", "utils.ts"), "utils");

    fs.mkdirSync(path.join(testDir, "tests"));
    fs.writeFileSync(path.join(testDir, "tests", "test.spec.ts"), "test");

    // Create node_modules (should be ignored)
    fs.mkdirSync(path.join(testDir, "node_modules"));
    fs.writeFileSync(path.join(testDir, "node_modules", "package.json"), "{}");

    // Create .git directory (should be ignored)
    fs.mkdirSync(path.join(testDir, ".git"));
    fs.writeFileSync(path.join(testDir, ".git", "config"), "config");
  });

  afterAll(() => {
    fs.rmSync(testDir, { recursive: true });
  });

  it("finds all files recursively by default", async () => {
    const result = await find({ path: testDir });
    
    expect(result.success).toBe(true);
    expect(result.files).toContain(path.join(testDir, "file1.txt"));
    expect(result.files).toContain(path.join(testDir, "file2.js"));
    expect(result.files).toContain(path.join(testDir, "README.md"));
    expect(result.files).toContain(path.join(testDir, "src", "index.ts"));
    expect(result.files).toContain(path.join(testDir, "src", "utils.ts"));
    
    // Should not include ignored directories
    expect(result.files.some(f => f.includes("node_modules"))).toBe(false);
    expect(result.files.some(f => f.includes(".git"))).toBe(false);
  });

  it("finds files matching a glob pattern", async () => {
    const result = await find({ path: testDir, pattern: "*.ts" });
    
    expect(result.success).toBe(true);
    expect(result.files).toContain(path.join(testDir, "src", "index.ts"));
    expect(result.files).toContain(path.join(testDir, "src", "utils.ts"));
    expect(result.files.some(f => f.endsWith(".txt"))).toBe(false);
    expect(result.files.some(f => f.endsWith(".js"))).toBe(false);
  });

  it("finds files matching a name pattern", async () => {
    const result = await find({ path: testDir, pattern: "README*" });
    
    expect(result.success).toBe(true);
    expect(result.files).toContain(path.join(testDir, "README.md"));
    expect(result.files.length).toBe(1);
  });

  it("finds files non-recursively when recursive is false", async () => {
    const result = await find({ path: testDir, recursive: false });
    
    expect(result.success).toBe(true);
    expect(result.files).toContain(path.join(testDir, "file1.txt"));
    expect(result.files).toContain(path.join(testDir, "file2.js"));
    expect(result.files.some(f => f.includes("src"))).toBe(false);
    expect(result.files.some(f => f.includes("tests"))).toBe(false);
  });

  it("returns error for non-existent path", async () => {
    const result = await find({ path: "/non/existent/path" });
    
    expect(result.success).toBe(false);
    expect(result.error).toBeDefined();
  });
});

describe("search", () => {
  let testDir: string;

  beforeAll(() => {
    // Create a test directory structure with content
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), "search-test-"));

    fs.writeFileSync(
      path.join(testDir, "file1.txt"),
      "Hello world\nThis is a test\nFoo bar baz"
    );

    fs.writeFileSync(
      path.join(testDir, "file2.js"),
      "function test() {\n  return 'test';\n}\n"
    );

    fs.mkdirSync(path.join(testDir, "src"));
    fs.writeFileSync(
      path.join(testDir, "src", "index.ts"),
      "export function hello() {\n  console.log('hello');\n}\n"
    );

    fs.writeFileSync(
      path.join(testDir, "src", "config.json"),
      '{"name": "test", "value": 123}'
    );

    // Create node_modules (should be ignored)
    fs.mkdirSync(path.join(testDir, "node_modules"));
    fs.writeFileSync(
      path.join(testDir, "node_modules", "test.js"),
      "module.exports = 'test';"
    );
  });

  afterAll(() => {
    fs.rmSync(testDir, { recursive: true });
  });

  it("searches for a simple string pattern", async () => {
    const result = await search({ pattern: "test", path: testDir });
    
    expect(result.success).toBe(true);
    expect(result.matches.length).toBeGreaterThan(0);
    
    const file1Match = result.matches.find(m => m.file.endsWith("file1.txt"));
    expect(file1Match).toBeDefined();
    expect(file1Match?.line).toContain("test");
    
    const file2Match = result.matches.find(m => m.file.endsWith("file2.js"));
    expect(file2Match).toBeDefined();
  });

  it("searches with a glob filter", async () => {
    const result = await search({ 
      pattern: "test", 
      path: testDir,
      glob: "*.txt"
    });
    
    expect(result.success).toBe(true);
    expect(result.matches.every(m => m.file.endsWith(".txt"))).toBe(true);
    expect(result.matches.some(m => m.file.endsWith(".js"))).toBe(false);
  });

  it("searches with multiple glob patterns", async () => {
    const result = await search({ 
      pattern: "function", 
      path: testDir,
      glob: "*.{js,ts}"
    });
    
    expect(result.success).toBe(true);
    expect(result.matches.every(m => 
      m.file.endsWith(".js") || m.file.endsWith(".ts")
    )).toBe(true);
  });

  it("includes line numbers in results", async () => {
    const result = await search({ pattern: "This is a test", path: testDir });
    
    expect(result.success).toBe(true);
    const match = result.matches[0];
    expect(match.lineNumber).toBeGreaterThan(0);
    expect(match.line).toContain("This is a test");
  });

  it("excludes ignored directories", async () => {
    const result = await search({ pattern: "module.exports", path: testDir });
    
    expect(result.success).toBe(true);
    expect(result.matches.some(m => m.file.includes("node_modules"))).toBe(false);
  });

  it("searches case-sensitive by default", async () => {
    const result = await search({ pattern: "HELLO", path: testDir });
    
    expect(result.success).toBe(true);
    expect(result.matches.length).toBe(0);
  });

  it("returns error for invalid path", async () => {
    const result = await search({ pattern: "test", path: "/non/existent" });
    
    expect(result.success).toBe(false);
    expect(result.error).toBeDefined();
  });

  it("uses current directory when path is not provided", async () => {
    const currentDir = process.cwd();
    const result = await search({ pattern: "describe" });
    
    expect(result.success).toBe(true);
    // Should find test files in the current directory
    expect(result.matches.some(m => m.file.includes(currentDir))).toBe(true);
  });
});
