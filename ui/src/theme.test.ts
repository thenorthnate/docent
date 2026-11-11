import { describe, expect, it } from "vitest";
import { followSystemTheme } from "./theme";

/** A stand-in for `matchMedia("(prefers-color-scheme: dark)")` we can flip. */
function fakeDarkQuery(matches: boolean) {
  const listeners = new Set<(event: { matches: boolean }) => void>();
  return {
    matches,
    addEventListener: (_: "change", fn: (event: { matches: boolean }) => void) => listeners.add(fn),
    removeEventListener: (_: "change", fn: (event: { matches: boolean }) => void) => listeners.delete(fn),
    change(next: boolean) {
      this.matches = next;
      listeners.forEach((fn) => fn({ matches: next }));
    },
  };
}

describe("followSystemTheme", () => {
  it("uses the dark tokens when the system prefers dark", () => {
    const root = document.createElement("html");
    followSystemTheme(root, fakeDarkQuery(true));
    expect(root.classList.contains("dark")).toBe(true);
  });

  it("uses the light tokens when the system prefers light", () => {
    const root = document.createElement("html");
    root.classList.add("dark");
    followSystemTheme(root, fakeDarkQuery(false));
    expect(root.classList.contains("dark")).toBe(false);
  });

  it("follows the system when it switches, until stopped", () => {
    const root = document.createElement("html");
    const query = fakeDarkQuery(false);
    const stop = followSystemTheme(root, query);

    query.change(true);
    expect(root.classList.contains("dark")).toBe(true);
    query.change(false);
    expect(root.classList.contains("dark")).toBe(false);

    stop();
    query.change(true);
    expect(root.classList.contains("dark")).toBe(false);
  });
});
