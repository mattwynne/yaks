import * as fs from "node:fs";
import * as path from "node:path";

export const BLOCKED_BASH_LOG_RELATIVE_PATH = ".pi/logs/protect-main-blocked-bash.jsonl";

export interface BlockedBashLogEntry {
  timestamp: string;
  cwd: string;
  command: string;
}

export function blockedBashLogPath(cwd = process.cwd()): string {
  return path.join(cwd, BLOCKED_BASH_LOG_RELATIVE_PATH);
}

export function appendBlockedBashLog(entry: BlockedBashLogEntry, logPath = blockedBashLogPath(entry.cwd)): void {
  fs.mkdirSync(path.dirname(logPath), { recursive: true });
  fs.appendFileSync(logPath, `${JSON.stringify(entry)}\n`, "utf8");
}

export function readRecentBlockedBashLog(logPath = blockedBashLogPath(), limit = 20): BlockedBashLogEntry[] {
  const boundedLimit = Math.max(0, Math.floor(limit));
  if (boundedLimit === 0 || !fs.existsSync(logPath)) return [];

  const lines = fs
    .readFileSync(logPath, "utf8")
    .split("\n")
    .filter((line) => line.trim().length > 0);

  return lines
    .slice(-boundedLimit)
    .map((line) => JSON.parse(line) as BlockedBashLogEntry);
}
