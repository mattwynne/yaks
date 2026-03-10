import * as fs from "node:fs";
import * as path from "node:path";
import { promisify } from "node:util";
import { minimatch } from "minimatch";

const readdir = promisify(fs.readdir);
const stat = promisify(fs.stat);
const readFile = promisify(fs.readFile);

// Directories and files to ignore (gitignore-style patterns)
const IGNORE_PATTERNS = [
  "node_modules",
  ".git",
  "target",
  "dist",
  "build",
  ".next",
  ".nuxt",
  ".output",
  ".cache",
  "coverage",
  ".DS_Store",
];

function shouldIgnore(filePath: string): boolean {
  const parts = filePath.split(path.sep);
  return IGNORE_PATTERNS.some((pattern) => parts.includes(pattern));
}

export interface FindParams {
  path: string;
  pattern?: string;
  recursive?: boolean;
}

export interface FindResult {
  success: boolean;
  files?: string[];
  error?: string;
}

export async function find(params: FindParams): Promise<FindResult> {
  const { path: searchPath, pattern, recursive = true } = params;

  try {
    // Check if path exists
    const pathStat = await stat(searchPath);
    if (!pathStat.isDirectory()) {
      return {
        success: false,
        error: `Path is not a directory: ${searchPath}`,
      };
    }

    const files: string[] = [];

    async function walk(dir: string) {
      if (shouldIgnore(dir)) return;

      const entries = await readdir(dir, { withFileTypes: true });

      for (const entry of entries) {
        const fullPath = path.join(dir, entry.name);

        if (shouldIgnore(fullPath)) continue;

        if (entry.isDirectory()) {
          if (recursive) {
            await walk(fullPath);
          }
        } else if (entry.isFile()) {
          // Apply pattern filter if provided
          if (pattern) {
            if (minimatch(entry.name, pattern) || minimatch(fullPath, pattern)) {
              files.push(fullPath);
            }
          } else {
            files.push(fullPath);
          }
        }
      }
    }

    await walk(searchPath);

    return {
      success: true,
      files: files.sort(),
    };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export interface SearchParams {
  pattern: string;
  path?: string;
  glob?: string;
}

export interface SearchMatch {
  file: string;
  lineNumber: number;
  line: string;
}

export interface SearchResult {
  success: boolean;
  matches?: SearchMatch[];
  error?: string;
}

export async function search(params: SearchParams): Promise<SearchResult> {
  const { pattern, path: searchPath = process.cwd(), glob } = params;

  try {
    // Check if path exists
    try {
      const pathStat = await stat(searchPath);
      if (!pathStat.isDirectory()) {
        return {
          success: false,
          error: `Path is not a directory: ${searchPath}`,
        };
      }
    } catch {
      return {
        success: false,
        error: `Path does not exist: ${searchPath}`,
      };
    }

    const matches: SearchMatch[] = [];
    const searchRegex = new RegExp(pattern);

    async function searchFiles(dir: string) {
      if (shouldIgnore(dir)) return;

      const entries = await readdir(dir, { withFileTypes: true });

      for (const entry of entries) {
        const fullPath = path.join(dir, entry.name);

        if (shouldIgnore(fullPath)) continue;

        if (entry.isDirectory()) {
          await searchFiles(fullPath);
        } else if (entry.isFile()) {
          // Apply glob filter if provided
          if (glob && !minimatch(entry.name, glob)) {
            continue;
          }

          try {
            const content = await readFile(fullPath, "utf-8");
            const lines = content.split("\n");

            lines.forEach((line, index) => {
              if (searchRegex.test(line)) {
                matches.push({
                  file: fullPath,
                  lineNumber: index + 1,
                  line: line.trim(),
                });
              }
            });
          } catch (error) {
            // Skip files that can't be read (binary files, permission issues, etc.)
            continue;
          }
        }
      }
    }

    await searchFiles(searchPath);

    return {
      success: true,
      matches,
    };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}
