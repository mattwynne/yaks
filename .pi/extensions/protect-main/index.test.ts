import { describe, it, expect, beforeAll, afterAll, vi } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import extension from "./index.js";

describe("Tool Registration Integration Tests", () => {
  let testDir: string;
  const registeredTools = new Map<string, any>();

  beforeAll(() => {
    // Create a test directory structure for real tool execution
    testDir = fs.mkdtempSync(path.join(os.tmpdir(), "tool-integration-test-"));

    // Create test files
    fs.writeFileSync(path.join(testDir, "file1.txt"), "Hello world\nThis is a test");
    fs.writeFileSync(path.join(testDir, "file2.js"), "function test() { return 42; }");
    fs.mkdirSync(path.join(testDir, "src"));
    fs.writeFileSync(path.join(testDir, "src", "index.ts"), "export const value = 'test';");

    // Create a mock ExtensionAPI
    const mockAPI: Partial<ExtensionAPI> = {
      registerTool: (tool: any) => {
        registeredTools.set(tool.name, tool);
      },
      on: vi.fn(),
    };

    // Execute the extension to register tools
    extension(mockAPI as ExtensionAPI);
  });

  afterAll(() => {
    fs.rmSync(testDir, { recursive: true });
  });

  describe("find tool", () => {
    it("is registered with an execute function", () => {
      const findTool = registeredTools.get("find");
      
      expect(findTool).toBeDefined();
      expect(findTool.name).toBe("find");
      expect(findTool.execute).toBeDefined();
      expect(typeof findTool.execute).toBe("function");
    });

    it("has correct tool metadata", () => {
      const findTool = registeredTools.get("find");
      
      expect(findTool.description).toBeDefined();
      expect(findTool.parameters).toBeDefined();
      expect(findTool.parameters.type).toBe("object");
      expect(findTool.parameters.required).toContain("path");
    });

    it("execute function has correct pi signature and returns valid content", async () => {
      const findTool = registeredTools.get("find");
      const toolCallId = "test-call-1";
      const params = { path: testDir };
      const signal = new AbortController().signal;
      const onUpdate = vi.fn();
      const ctx = {};

      // Call execute with pi signature
      const result = await findTool.execute(toolCallId, params, signal, onUpdate, ctx);

      // Verify return shape
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(Array.isArray(result.content)).toBe(true);
      expect(result.content.length).toBeGreaterThan(0);
      expect(result.content[0]).toHaveProperty("type", "text");
      expect(result.content[0]).toHaveProperty("text");
      expect(typeof result.content[0].text).toBe("string");
      
      // Verify content includes our test files
      const text = result.content[0].text;
      expect(text).toContain("file1.txt");
      expect(text).toContain("file2.js");
      expect(text).toContain("index.ts");
    });

    it("execute function works with pattern parameter", async () => {
      const findTool = registeredTools.get("find");
      const params = { path: testDir, pattern: "*.ts" };
      const signal = new AbortController().signal;

      const result = await findTool.execute("call-2", params, signal, vi.fn(), {});

      expect(result.content[0].text).toContain("index.ts");
      expect(result.content[0].text).not.toContain("file1.txt");
      expect(result.content[0].text).not.toContain("file2.js");
    });

    it("execute function returns error shape for invalid path", async () => {
      const findTool = registeredTools.get("find");
      const params = { path: "/non/existent/path/12345" };
      const signal = new AbortController().signal;

      const result = await findTool.execute("call-3", params, signal, vi.fn(), {});

      // Verify error return shape
      expect(result.isError).toBe(true);
      expect(result.content).toBeDefined();
      expect(Array.isArray(result.content)).toBe(true);
      expect(result.content[0]).toHaveProperty("type", "text");
      expect(result.content[0].text).toBeTruthy();
    });
  });

  describe("search tool", () => {
    it("is registered with an execute function", () => {
      const searchTool = registeredTools.get("search");
      
      expect(searchTool).toBeDefined();
      expect(searchTool.name).toBe("search");
      expect(searchTool.execute).toBeDefined();
      expect(typeof searchTool.execute).toBe("function");
    });

    it("has correct tool metadata", () => {
      const searchTool = registeredTools.get("search");
      
      expect(searchTool.description).toBeDefined();
      expect(searchTool.parameters).toBeDefined();
      expect(searchTool.parameters.type).toBe("object");
      expect(searchTool.parameters.required).toContain("pattern");
    });

    it("execute function has correct pi signature and returns valid content", async () => {
      const searchTool = registeredTools.get("search");
      const toolCallId = "test-call-4";
      const params = { pattern: "test", path: testDir };
      const signal = new AbortController().signal;
      const onUpdate = vi.fn();
      const ctx = {};

      // Call execute with pi signature
      const result = await searchTool.execute(toolCallId, params, signal, onUpdate, ctx);

      // Verify return shape
      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(Array.isArray(result.content)).toBe(true);
      expect(result.content[0]).toHaveProperty("type", "text");
      expect(result.content[0]).toHaveProperty("text");
      expect(typeof result.content[0].text).toBe("string");
      
      // Verify content includes matches with line numbers
      const text = result.content[0].text;
      expect(text).toContain("file1.txt");
      expect(text).toContain("test");
      expect(text).toMatch(/:\d+:/); // Line number format
    });

    it("execute function works with glob parameter", async () => {
      const searchTool = registeredTools.get("search");
      const params = { pattern: "test", path: testDir, glob: "*.txt" };
      const signal = new AbortController().signal;

      const result = await searchTool.execute("call-5", params, signal, vi.fn(), {});

      const text = result.content[0].text;
      expect(text).toContain("file1.txt");
      expect(text).not.toContain(".js");
      expect(text).not.toContain(".ts");
    });

    it("execute function returns error shape for invalid path", async () => {
      const searchTool = registeredTools.get("search");
      const params = { pattern: "test", path: "/non/existent/path/12345" };
      const signal = new AbortController().signal;

      const result = await searchTool.execute("call-6", params, signal, vi.fn(), {});

      // Verify error return shape
      expect(result.isError).toBe(true);
      expect(result.content).toBeDefined();
      expect(Array.isArray(result.content)).toBe(true);
      expect(result.content[0]).toHaveProperty("type", "text");
      expect(result.content[0].text).toBeTruthy();
    });

    it("execute function handles empty results", async () => {
      const searchTool = registeredTools.get("search");
      const params = { pattern: "NONEXISTENTPATTERN12345", path: testDir };
      const signal = new AbortController().signal;

      const result = await searchTool.execute("call-7", params, signal, vi.fn(), {});

      expect(result.content[0].type).toBe("text");
      expect(result.content[0].text).toBe("");
    });
  });

  describe("Tool registration verification", () => {
    it("both tools use 'execute' not 'handler'", () => {
      const findTool = registeredTools.get("find");
      const searchTool = registeredTools.get("search");

      // This test would catch the bug where 'handler' was used instead of 'execute'
      expect(findTool).not.toHaveProperty("handler");
      expect(searchTool).not.toHaveProperty("handler");
      
      expect(findTool).toHaveProperty("execute");
      expect(searchTool).toHaveProperty("execute");
    });

    it("tools are properly registered when extension initializes", () => {
      // Verify tools were registered during extension initialization
      expect(registeredTools.size).toBeGreaterThanOrEqual(2);
      expect(registeredTools.has("find")).toBe(true);
      expect(registeredTools.has("search")).toBe(true);
    });
  });
});
