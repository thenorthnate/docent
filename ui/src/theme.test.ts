import { describe, expect, it } from "vitest";
import { createThemeSetting, followSystemTheme, THEME_STORAGE_KEY } from "./theme";

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

/** An in-memory stand-in for `localStorage`. */
function fakeStorage(initial: Record<string, string> = {}) {
  const items = new Map(Object.entries(initial));
  return {
    getItem: (key: string) => items.get(key) ?? null,
    setItem: (key: string, value: string) => void items.set(key, value),
    items,
  };
}

describe("createThemeSetting", () => {
  it("defaults to System and follows the OS live", () => {
    const root = document.createElement("html");
    const query = fakeDarkQuery(true);
    const theme = createThemeSetting({ root, query, storage: fakeStorage() });

    expect(theme.choice.value).toBe("system");
    expect(root.classList.contains("dark")).toBe(true);
    query.change(false);
    expect(root.classList.contains("dark")).toBe(false);
  });

  it.each([
    ["light", true, false],
    ["dark", false, true],
  ] as const)("applies a stored %s choice whatever the OS says", (stored, osDark, dark) => {
    const root = document.createElement("html");
    const query = fakeDarkQuery(osDark);
    const theme = createThemeSetting({ root, query, storage: fakeStorage({ [THEME_STORAGE_KEY]: stored }) });

    expect(theme.choice.value).toBe(stored);
    expect(root.classList.contains("dark")).toBe(dark);
    query.change(!osDark);
    expect(root.classList.contains("dark")).toBe(dark);
  });

  it("falls back to System when the stored value is unknown", () => {
    const root = document.createElement("html");
    const theme = createThemeSetting({
      root,
      query: fakeDarkQuery(true),
      storage: fakeStorage({ [THEME_STORAGE_KEY]: "sepia" }),
    });

    expect(theme.choice.value).toBe("system");
    expect(root.classList.contains("dark")).toBe(true);
  });

  it("falls back to System when storage can't be read", () => {
    const root = document.createElement("html");
    const storage = {
      getItem: () => {
        throw new Error("storage disabled");
      },
      setItem: () => {
        throw new Error("storage disabled");
      },
    };
    const theme = createThemeSetting({ root, query: fakeDarkQuery(true), storage });

    expect(theme.choice.value).toBe("system");
    expect(root.classList.contains("dark")).toBe(true);
    theme.set("light");
    expect(root.classList.contains("dark")).toBe(false);
  });

  it("switches between choices, saving each and stopping or resuming OS-following", () => {
    const root = document.createElement("html");
    const query = fakeDarkQuery(false);
    const storage = fakeStorage();
    const theme = createThemeSetting({ root, query, storage });

    theme.set("dark");
    expect(theme.choice.value).toBe("dark");
    expect(root.classList.contains("dark")).toBe(true);
    expect(storage.items.get(THEME_STORAGE_KEY)).toBe("dark");
    query.change(false);
    expect(root.classList.contains("dark")).toBe(true);

    theme.set("light");
    expect(root.classList.contains("dark")).toBe(false);
    expect(storage.items.get(THEME_STORAGE_KEY)).toBe("light");

    query.change(true);
    theme.set("system");
    expect(storage.items.get(THEME_STORAGE_KEY)).toBe("system");
    expect(root.classList.contains("dark")).toBe(true);
    query.change(false);
    expect(root.classList.contains("dark")).toBe(false);
  });
});
