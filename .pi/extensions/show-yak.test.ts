/**
 * Integration tests for the show-yak extension.
 *
 * Uses the pi SDK to load the extension and invoke the tool directly,
 * running against the real `yx` CLI in this repo.
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import {
  createAgentSession,
  DefaultResourceLoader,
  SessionManager,
  SettingsManager,
} from "@mariozechner/pi-coding-agent";
import type { AgentSession } from "@mariozechner/pi-coding-agent";
import showYakExtension from "./show-yak.js";

// Helper: find the show_yak tool and call execute
async function callShowYak(session: AgentSession, name: string) {
  const tool = session.agent.state.tools.find((t) => t.name === "show_yak");
  if (!tool) throw new Error("show_yak tool not registered");
  const result = await tool.execute("test-call-id", { name }, new AbortController().signal);
  return result;
}

describe("show_yak tool", () => {
  let session: AgentSession;

  beforeAll(async () => {
    const loader = new DefaultResourceLoader({
      cwd: process.cwd(),
      extensionFactories: [showYakExtension],
      // Disable discovery of other extensions to isolate the test
      skipDiscovery: true,
    });
    await loader.reload();

    ({ session } = await createAgentSession({
      cwd: process.cwd(),
      resourceLoader: loader,
      sessionManager: SessionManager.inMemory(),
      settingsManager: SettingsManager.inMemory(),
      tools: [], // no built-in tools needed
    }));
  });

  afterAll(() => {
    session?.dispose();
  });

  it("is registered as a tool", () => {
    const toolNames = session.agent.state.tools.map((t) => t.name);
    expect(toolNames).toContain("show_yak");
  });

  it("returns JSON for an existing yak", async () => {
    const result = await callShowYak(session, "dx");

    expect(result.isError).toBeFalsy();
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toBeDefined();

    const parsed = JSON.parse(text!);
    expect(parsed.name).toBe("dx");
    expect(parsed).toHaveProperty("id");
    expect(parsed).toHaveProperty("state");
    expect(parsed).toHaveProperty("children");
  });

  it("returns an error for a non-existent yak", async () => {
    const result = await callShowYak(session, "this yak does not exist at all");

    expect(result.isError).toBe(true);
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toMatch(/error/i);
  });

  it("splits multi-word names into separate args", async () => {
    const result = await callShowYak(session, "protect main from direct changes");

    expect(result.isError).toBeFalsy();
    const text = result.content.find((c: any) => c.type === "text")?.text;
    expect(text).toBeDefined();

    const parsed = JSON.parse(text!);
    expect(parsed.name).toBe("protect main from direct changes");
  });
});
