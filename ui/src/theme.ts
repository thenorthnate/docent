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
