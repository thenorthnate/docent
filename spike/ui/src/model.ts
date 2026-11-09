// Shared data model. In Docent proper, hunks and moves come from docent-git; here
// they come from diff.ts / moves.ts (browser fixtures) or from `git diff` via Tauri.

export type Side = 'old' | 'new';

/** One git hunk, 1-based lines. A count of 0 means a pure insertion or deletion. */
export interface Hunk {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
}

export interface FileDiff {
  path: string;
  original: string;
  modified: string;
  hunks: Hunk[];
}

export interface Range {
  path: string;
  start: number; // 1-based, inclusive
  end: number; // inclusive
}

export interface Move {
  id: number;
  kind: 'moved' | 'moved-changed';
  /** Lines removed, on the old side. */
  from: Range;
  /** Lines added, on the new side. */
  to: Range;
  /** For moved-changed: destination lines that differ from the source. */
  changed?: number[];
}

export interface Annotation {
  path: string;
  side: Side;
  line: number;
  kind: 'finding' | 'comment';
  severity?: 'high' | 'medium' | 'low';
  text: string;
}

export interface Fixture {
  name: string;
  files: FileDiff[];
  annotations: Annotation[];
  moves: Move[];
}

export interface ViewerCallbacks {
  onExplain(path: string, side: Side, line: number): void;
  onJump(path: string, side: Side, line: number): void;
  log(msg: string): void;
}

export interface MountOptions extends ViewerCallbacks {
  mode: 'split' | 'unified';
  annotations: Annotation[];
  moves: Move[];
}

export interface MountStats {
  /** mount() call until the editor exists in the DOM. */
  createMs: number;
  /** mount() call until the diff is computed and decorations applied. */
  readyMs: number;
  /** Hunks the viewer shows that differ from the git hunks it was given (Monaco recomputes). */
  hunkMismatches: number;
  viewerHunks: number;
  /** Up to a few differing hunk keys, `v:` from the viewer, `g:` from git. */
  mismatchDetail: string[];
}

export interface ScrollStats {
  frames: number;
  meanMs: number;
  p95Ms: number;
  maxMs: number;
  /** Frames slower than 33 ms (below 30 fps). */
  longFrames: number;
}

export interface Viewer {
  readonly name: 'monaco' | 'codemirror';
  mount(el: HTMLElement, file: FileDiff, opts: MountOptions): Promise<MountStats>;
  reveal(side: Side, line: number): void;
  /** Scroll top to bottom, one step per frame, and measure frame times. */
  scrollBench(pxPerFrame: number, maxFrames: number): Promise<ScrollStats>;
  dispose(): void;
}

export function lineCount(text: string): number {
  if (text === '') return 0;
  let n = 1;
  for (let i = 0; i < text.length; i++) if (text.charCodeAt(i) === 10) n++;
  return text.endsWith('\n') ? n - 1 : n;
}

export function frameStats(deltas: number[]): ScrollStats {
  const sorted = [...deltas].sort((a, b) => a - b);
  const mean = deltas.reduce((s, d) => s + d, 0) / Math.max(1, deltas.length);
  const pick = (q: number) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))] ?? 0;
  return {
    frames: deltas.length,
    meanMs: round(mean),
    p95Ms: round(pick(0.95)),
    maxMs: round(sorted[sorted.length - 1] ?? 0),
    longFrames: deltas.filter((d) => d > 33).length,
  };
}

export function round(n: number): number {
  return Math.round(n * 10) / 10;
}

export function nextFrame(): Promise<number> {
  return new Promise((r) => requestAnimationFrame(r));
}

/** Next frame, or a timeout when the page is hidden and rAF is paused. */
export function settle(): Promise<unknown> {
  return Promise.race([nextFrame(), new Promise((r) => setTimeout(r, 250))]);
}

/** Compare viewer hunk keys with git hunk keys. */
export function compareHunks(viewer: string[], git: string[]): { hunkMismatches: number; mismatchDetail: string[] } {
  const v = viewer.filter((k) => !git.includes(k)).map((k) => `v:${k}`);
  const g = git.filter((k) => !viewer.includes(k)).map((k) => `g:${k}`);
  return { hunkMismatches: v.length + g.length, mismatchDetail: [...v, ...g].slice(0, 6) };
}
