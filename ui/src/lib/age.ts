const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK = 7 * DAY;
const MONTH = 30 * DAY;
const YEAR = 365 * DAY;

/** The largest unit first, so the age reads in the biggest whole unit. */
const UNITS: Array<[number, string]> = [
  [YEAR, "y"],
  [MONTH, "mo"],
  [WEEK, "w"],
  [DAY, "d"],
  [HOUR, "h"],
  [MINUTE, "m"],
];

/**
 * How long ago `timestamp` was, compactly ("5m", "3h", "3d", "3w", "2mo", "1y"), as
 * an inbox row shows it. The timestamp comes from the forge, so one that
 * can't be read shows nothing, and one in the future (clock skew) reads as
 * "just now".
 */
export function formatAge(timestamp: string, now: Date = new Date()): string {
  const then = Date.parse(timestamp);
  if (Number.isNaN(then)) return "";
  const elapsed = now.getTime() - then;
  for (const [size, suffix] of UNITS) {
    if (elapsed >= size) return `${Math.floor(elapsed / size)}${suffix}`;
  }
  return "just now";
}
