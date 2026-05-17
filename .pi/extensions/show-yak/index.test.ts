/**
 * Tests for the show-yak extension.
 *
 * Loads the extension against a tiny fake ExtensionAPI and invokes the
 * registered tools directly.
 */

import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import showYakExtension, { limitTextByLines } from "./index.js";

type RegisteredTool = {
  name: string;
  execute: (toolCallId: string, params: any, signal?: AbortSignal) => Promise<any>;
};

type ExecCall = { cmd: string; args: string[] };

const tools: RegisteredTool[] = [];
const execCalls: ExecCall[] = [];

function makeLines(prefix: string, count: number): string {
  return Array.from({ length: count }, (_, i) => `${prefix} ${i + 1}`).join("\n");
}

async function fakeExec(cmd: string, args: string[]) {
  execCalls.push({ cmd, args });

  if (cmd === "bin/dev") {
    if (args[0] === "merge" && args[1] === "success-branch") {
      return { code: 0, stdout: `${makeLines("cargo check log", 300)}\nMerged success-branch\n`, stderr: "" };
    }
    if (args[0] === "merge" && args[1] === "failing-branch") {
      return { code: 1, stdout: makeLines("stdout line", 150), stderr: makeLines("stderr line", 10) };
    }
    return { code: 1, stdout: "", stderr: `unexpected bin/dev command: ${args.join(" ")}` };
  }

  if (cmd !== "yx") {
    return { code: 1, stdout: "", stderr: `unexpected command: ${cmd}` };
  }

  if (args[0] === "long-output") {
    return { code: 0, stdout: makeLines("yx output line", 250), stderr: "" };
  }

  if (args[0] === "list") {
    if (args.includes("foobar")) {
      return { code: 1, stdout: "", stderr: "Unknown filter: foobar" };
    }
    return { code: 0, stdout: "[]\n", stderr: "" };
  }

  if (args[0] === "show") {
    const name = args.slice(3).join(" ");
    if (name === "this yak does not exist at all") {
      return { code: 1, stdout: "", stderr: "yak not found" };
    }
    return {
      code: 0,
      stdout: JSON.stringify({ name, state: "todo", children: [] }),
      stderr: "",
    };
  }

  return { code: 1, stdout: "", stderr: "unexpected yx command" };
}

beforeAll(() => {
  tools.length = 0;
  showYakExtension({
    registerTool(tool: RegisteredTool) {
      tools.push(tool);
    },
    exec: fakeExec,
  } as any);
});

beforeEach(() => {
  execCalls.length = 0;
});

function toolNamed(name: string): RegisteredTool {
  const tool = tools.find((t) => t.name === name);
  if (!tool) throw new Error(`${name} tool not registered`);
  return tool;
}

async function callShowYak(name: string) {
  return toolNamed("show_yak").execute("test-call-id", { name }, new AbortController().signal);
}

async function callListYaks(params: Record<string, string> = {}) {
  return toolNamed("list_yaks").execute("test-call-id", params, new AbortController().signal);
}

async function callYx(params: { args: string[]; timeoutMs?: number }) {
  return toolNamed("yx").execute("test-call-id", params, new AbortController().signal);
}

async function callMergeYak(branch: string) {
  return toolNamed("merge_yak").execute("test-call-id", { branch }, new AbortController().signal);
}

describe("limitTextByLines", () => {
  it("returns the tail with a truncation notice when text is too long", () => {
    const result = limitTextByLines(makeLines("line", 5), 3);

    expect(result.truncated).toBe(true);
    expect(result.totalLines).toBe(5);
    expect(result.text).toBe("[output truncated to last 3 of 5 lines]\nline 3\nline 4\nline 5");
  });
});

describe("yx tool", () => {
  it("is registered as a tool", () => {
    expect(tools.map((t) => t.name)).toContain("yx");
  });

  it("runs yx with structured args", async () => {
    const result = await callYx({ args: ["list", "--format", "json"] });

    expect(result.isError).toBeFalsy();
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(JSON.parse(text!)).toEqual([]);
    expect(execCalls[0]).toEqual({ cmd: "yx", args: ["list", "--format", "json"] });
  });

  it("returns an error for nonzero exits", async () => {
    const result = await callYx({ args: ["unknown"] });

    expect(result.isError).toBe(true);
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toMatch(/unexpected yx command/i);
  });

  it("truncates long generic output", async () => {
    const result = await callYx({ args: ["long-output"] });

    expect(result.isError).toBeFalsy();
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toContain("[output truncated to last 200 of 250 lines]");
    expect(text!.split("\n")).not.toContain("yx output line 1");
    expect(text).toContain("yx output line 250");
  });
});

describe("list_yaks tool", () => {
  it("is registered as a tool", () => {
    expect(tools.map((t) => t.name)).toContain("list_yaks");
  });

  it("returns valid JSON array", async () => {
    const result = await callListYaks();

    expect(result.isError).toBeFalsy();
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(JSON.parse(text!)).toEqual([]);
    expect(execCalls[0]).toEqual({ cmd: "yx", args: ["list", "--format", "json"] });
  });

  it("passes --only flag when only param is provided", async () => {
    const result = await callListYaks({ only: "not-done" });

    expect(result.isError).toBeFalsy();
    expect(execCalls[0]).toEqual({ cmd: "yx", args: ["list", "--format", "json", "--only", "not-done"] });
  });

  it("passes --tag flag when tag param is provided", async () => {
    const result = await callListYaks({ tag: "ready" });

    expect(result.isError).toBeFalsy();
    expect(execCalls[0]).toEqual({ cmd: "yx", args: ["list", "--format", "json", "--tag", "ready"] });
  });

  it("returns error for invalid filter", async () => {
    const result = await callListYaks({ only: "foobar" });

    expect(result.isError).toBe(true);
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toMatch(/Unknown filter/i);
  });
});

describe("show_yak tool", () => {
  it("is registered as a tool", () => {
    expect(tools.map((t) => t.name)).toContain("show_yak");
  });

  it("returns JSON for an existing yak", async () => {
    const result = await callShowYak("dx");

    expect(result.isError).toBeFalsy();
    const text = result.content.find((c: any) => c.type === "text")?.text;
    const parsed = JSON.parse(text!);
    expect(parsed.name).toBe("dx");
  });

  it("returns an error for a non-existent yak", async () => {
    const result = await callShowYak("this yak does not exist at all");

    expect(result.isError).toBe(true);
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toMatch(/error/i);
  });

  it("splits multi-word names into separate args", async () => {
    const result = await callShowYak("update pi extensions for earendil package rename");

    expect(result.isError).toBeFalsy();
    expect(execCalls[0]).toEqual({
      cmd: "yx",
      args: [
        "show",
        "--format",
        "json",
        "update",
        "pi",
        "extensions",
        "for",
        "earendil",
        "package",
        "rename",
      ],
    });
  });
});

describe("merge_yak tool", () => {
  it("returns a concise success message without verbose check logs", async () => {
    const result = await callMergeYak("success-branch");

    expect(result.isError).toBeFalsy();
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toBe("Merged yak branch 'success-branch' successfully.");
    expect(text).not.toContain("cargo check log");
    expect(execCalls[0]).toEqual({ cmd: "bin/dev", args: ["merge", "success-branch"] });
  });

  it("returns a truncated tail on failure", async () => {
    const result = await callMergeYak("failing-branch");

    expect(result.isError).toBe(true);
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toContain("Merge failed for branch 'failing-branch'. Showing last 120 of 160 lines:");
    expect(text).toContain("[output truncated to last 120 of 160 lines]");
    expect(text!.split("\n")).not.toContain("stdout line 1");
    expect(text).toContain("stdout line 41");
    expect(text).toContain("stderr line 10");
  });
});
