/**
 * The message to show for a failed Tauri command. The Rust core sends errors
 * as `{ kind, message }`; anything else is shown as text.
 */
export function messageOf(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return String(err.message);
  }
  return String(err);
}
