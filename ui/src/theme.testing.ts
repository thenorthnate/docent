// Test doubles for the theme setting. Only test files import this module.
import { createThemeSetting, type DarkQuery, type ThemeSetting } from "./theme";

/** A stand-in for `matchMedia("(prefers-color-scheme: dark)")` we can flip. */
export function fakeDarkQuery(matches: boolean) {
  const listeners = new Set<(event: { matches: boolean }) => void>();
  const query = {
    matches,
    addEventListener: (_: "change", fn: (event: { matches: boolean }) => void) => void listeners.add(fn),
    removeEventListener: (_: "change", fn: (event: { matches: boolean }) => void) => void listeners.delete(fn),
    /** Flips the OS theme, as the user changing it in system settings would. */
    change(next: boolean) {
      query.matches = next;
      listeners.forEach((fn) => fn({ matches: next }));
    },
  };
  return query satisfies DarkQuery;
}

/** An in-memory stand-in for `localStorage`. */
export function fakeStorage(initial: Record<string, string> = {}) {
  const items = new Map(Object.entries(initial));
  return {
    getItem: (key: string) => items.get(key) ?? null,
    setItem: (key: string, value: string) => void items.set(key, value),
    items,
  };
}

/** A theme setting on a detached `<html>`, with a light OS and nothing stored. */
export function testThemeSetting(): ThemeSetting {
  return createThemeSetting({
    root: document.createElement("html"),
    query: fakeDarkQuery(false),
    storage: fakeStorage(),
  });
}
