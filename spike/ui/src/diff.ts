// Line diff used as a stand-in for `git diff -U0` on browser fixtures.
// Plain Myers with common prefix/suffix trimming. Fine for small files; the large
// fixture generator emits its own hunks instead.
import type { Hunk } from './model';

export function splitLines(text: string): string[] {
  if (text === '') return [];
  const lines = text.split('\n');
  if (lines[lines.length - 1] === '') lines.pop();
  return lines;
}

export function lineDiff(original: string, modified: string): Hunk[] {
  const a = splitLines(original);
  const b = splitLines(modified);
  let pre = 0;
  while (pre < a.length && pre < b.length && a[pre] === b[pre]) pre++;
  let suf = 0;
  while (suf < a.length - pre && suf < b.length - pre && a[a.length - 1 - suf] === b[b.length - 1 - suf]) suf++;
  const A = a.slice(pre, a.length - suf);
  const B = b.slice(pre, b.length - suf);
  const ops = myers(A, B); // 'e' equal, 'd' delete, 'i' insert

  const hunks: Hunk[] = [];
  let ai = 0;
  let bi = 0;
  let cur: Hunk | null = null;
  for (const op of ops) {
    if (op === 'e') {
      if (cur) hunks.push(cur);
      cur = null;
      ai++;
      bi++;
      continue;
    }
    if (!cur) cur = { oldStart: pre + ai + 1, oldLines: 0, newStart: pre + bi + 1, newLines: 0 };
    if (op === 'd') {
      cur.oldLines++;
      ai++;
    } else {
      cur.newLines++;
      bi++;
    }
  }
  if (cur) hunks.push(cur);
  // git convention: a zero-length side's start is the line *before* the gap.
  for (const h of hunks) {
    if (h.oldLines === 0) h.oldStart--;
    if (h.newLines === 0) h.newStart--;
  }
  return hunks;
}

function myers(a: string[], b: string[]): ('e' | 'd' | 'i')[] {
  const n = a.length;
  const m = b.length;
  const max = n + m;
  const v = new Int32Array(2 * max + 2);
  const trace: Int32Array[] = [];
  outer: for (let d = 0; d <= max; d++) {
    trace.push(v.slice());
    for (let k = -d; k <= d; k += 2) {
      let x = k === -d || (k !== d && v[max + k - 1] < v[max + k + 1]) ? v[max + k + 1] : v[max + k - 1] + 1;
      let y = x - k;
      while (x < n && y < m && a[x] === b[y]) {
        x++;
        y++;
      }
      v[max + k] = x;
      if (x >= n && y >= m) break outer;
    }
  }
  const ops: ('e' | 'd' | 'i')[] = [];
  let x = n;
  let y = m;
  for (let d = trace.length - 1; d > 0; d--) {
    const vd = trace[d];
    const k = x - y;
    const prevK = k === -d || (k !== d && vd[max + k - 1] < vd[max + k + 1]) ? k + 1 : k - 1;
    const prevX = vd[max + prevK];
    const prevY = prevX - prevK;
    while (x > prevX && y > prevY) {
      ops.push('e');
      x--;
      y--;
    }
    ops.push(x === prevX ? 'i' : 'd');
    x = prevX;
    y = prevY;
  }
  while (x > 0 && y > 0) {
    ops.push('e');
    x--;
    y--;
  }
  return ops.reverse();
}

/** The new-side line after which an old-side line's deletion is shown in unified views. */
export function newLineBefore(hunks: Hunk[], oldLine: number): number {
  let offset = 0;
  for (const h of hunks) {
    const oldEnd = h.oldStart + h.oldLines; // exclusive
    if (h.oldLines > 0 && oldLine >= h.oldStart && oldLine < oldEnd) return h.newLines > 0 ? h.newStart - 1 : h.newStart;
    if (h.oldLines > 0 ? oldEnd <= oldLine : h.oldStart < oldLine) offset += h.newLines - h.oldLines;
  }
  return oldLine - 1 + offset;
}
