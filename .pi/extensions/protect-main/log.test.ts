import { describe, it, expect } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  appendBlockedBashLog,
  readRecentBlockedBashLog,
  BLOCKED_BASH_LOG_RELATIVE_PATH,
} from "./log.js";

describe("blocked bash log", () => {
  it("returns an empty array when the log file does not exist", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "blocked-bash-log-missing-"));
    try {
      expect(readRecentBlockedBashLog(path.join(dir, BLOCKED_BASH_LOG_RELATIVE_PATH))).toEqual([]);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it("appends JSONL records and reads the most recent entries", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "blocked-bash-log-"));
    const logPath = path.join(dir, BLOCKED_BASH_LOG_RELATIVE_PATH);
    try {
      appendBlockedBashLog({ timestamp: "2026-05-16T00:00:00.000Z", cwd: dir, command: "first" }, logPath);
      appendBlockedBashLog({ timestamp: "2026-05-16T00:00:01.000Z", cwd: dir, command: "second" }, logPath);
      appendBlockedBashLog({ timestamp: "2026-05-16T00:00:02.000Z", cwd: dir, command: "third" }, logPath);

      expect(readRecentBlockedBashLog(logPath, 2).map((entry) => entry.command)).toEqual(["second", "third"]);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });
});
