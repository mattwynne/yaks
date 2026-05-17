import { describe, it, expect } from "vitest";
import { clampMaxCount, limitLines } from "./index.js";

describe("limitLines", () => {
  it("returns all lines when no limit is provided", () => {
    expect(limitLines("a\nb\nc")).toBe("a\nb\nc");
  });

  it("truncates to the first N lines", () => {
    expect(limitLines("a\nb\nc", 2)).toBe("a\nb");
  });

  it("treats negative limits as zero", () => {
    expect(limitLines("a\nb", -1)).toBe("");
  });
});

describe("clampMaxCount", () => {
  it("uses the default when omitted", () => {
    expect(clampMaxCount(undefined, 5)).toBe(5);
  });

  it("caps the value at 100", () => {
    expect(clampMaxCount(101, 5)).toBe(100);
  });

  it("does not allow negative values", () => {
    expect(clampMaxCount(-1, 5)).toBe(0);
  });
});
