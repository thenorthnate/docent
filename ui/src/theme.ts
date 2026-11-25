import { readonly, ref, type InjectionKey, type Ref } from "vue";

/** The parts of a `MediaQueryList` that theme syncing needs. */
export interface DarkQuery {
  readonly matches: boolean;
  addEventListener(type: "change", listener: (event: { matches: boolean }) => void): void;
  removeEventListener(type: "change", listener: (event: { matches: boolean }) => void): void;
}

/**
 * Keeps shadcn-vue's `dark` class on `root` in step with the system theme.
 * Returns a function that stops following it.
 */
export function followSystemTheme(
  root: HTMLElement = document.documentElement,
  query: DarkQuery = window.matchMedia("(prefers-color-scheme: dark)"),
): () => void {
  const apply = (event: { matches: boolean }) => {
    root.classList.toggle("dark", event.matches);
  };
  apply(query);
  query.addEventListener("change", apply);
  return () => query.removeEventListener("change", apply);
}

/** The Appearance setting: follow the OS, or pin light or dark. */
export type ThemeChoice = "system" | "light" | "dark";

export const THEME_CHOICES: readonly ThemeChoice[] = ["system", "light", "dark"];

/** Where the choice lives in `localStorage`. */
export const THEME_STORAGE_KEY = "docent.theme";

/** The parts of `Storage` the theme setting needs. */
export type ThemeStorage = Pick<Storage, "getItem" | "setItem">;

/** The live Appearance setting that `createThemeSetting` returns. */
export interface ThemeSetting {
  readonly choice: Readonly<Ref<ThemeChoice>>;
  /** Applies `choice` now and saves it for the next launch. */
  set(choice: ThemeChoice): void;
}

export const themeSettingKey: InjectionKey<ThemeSetting> = Symbol("themeSetting");

function isThemeChoice(value: unknown): value is ThemeChoice {
  return THEME_CHOICES.includes(value as ThemeChoice);
}

function readChoice(storage: ThemeStorage): ThemeChoice {
  try {
    const stored = storage.getItem(THEME_STORAGE_KEY);
    return isThemeChoice(stored) ? stored : "system";
  } catch {
    // Storage can be disabled; the OS theme is a safe default.
    return "system";
  }
}

/**
 * Applies the stored Appearance choice to `root` straight away, synchronously,
 * so calling it before Vue mounts avoids a flash of the wrong theme.
 */
export function createThemeSetting({
  root = document.documentElement,
  query = window.matchMedia("(prefers-color-scheme: dark)"),
  storage = window.localStorage,
}: { root?: HTMLElement; query?: DarkQuery; storage?: ThemeStorage } = {}): ThemeSetting {
  const choice = ref<ThemeChoice>(readChoice(storage));
  let stopFollowing: (() => void) | null = null;

  const apply = () => {
    stopFollowing?.();
    stopFollowing = null;
    if (choice.value === "system") {
      stopFollowing = followSystemTheme(root, query);
    } else {
      root.classList.toggle("dark", choice.value === "dark");
    }
  };

  apply();
  return {
    choice: readonly(choice),
    set(next) {
      choice.value = next;
      apply();
      try {
        storage.setItem(THEME_STORAGE_KEY, next);
      } catch (error) {
        console.error("Could not save the theme choice", error);
      }
    },
  };
}
