import type { Hunk } from '../model';

/** Canonical key for comparing a viewer's hunks with git's (git's zero-length conventions). */
export function hunkKeyOf(h: Hunk): string {
  return `${h.oldStart},${h.oldLines},${h.newStart},${h.newLines}`;
}
