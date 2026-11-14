import { describe, expect, it } from "vitest";
import { formatAge } from "./age";

const now = new Date("2026-11-13T12:00:00Z");

describe("formatAge", () => {
  it.each([
    ["2026-11-13T11:59:30Z", "just now"],
    ["2026-11-13T11:55:00Z", "5m"],
    ["2026-11-13T09:00:00Z", "3h"],
    ["2026-11-10T12:00:00Z", "3d"],
    ["2026-10-01T12:00:00Z", "6w"],
    ["2025-11-13T12:00:00Z", "1y"],
  ])("an item opened at %s is %s old", (openedAt, age) => {
    expect(formatAge(openedAt, now)).toBe(age);
  });

  it("treats a time in the future, from clock skew, as just now", () => {
    expect(formatAge("2026-11-13T12:05:00Z", now)).toBe("just now");
  });

  it("shows nothing for a timestamp it can't read", () => {
    expect(formatAge("yesterday", now)).toBe("");
  });
});
