// Moved-block detection, standing in for docent-git (PLAN §5.14). Roughly git's
// `--color-moved=blocks`: exact runs of >= 3 removed lines that reappear as added
// lines, in any file. A second pass pairs leftover runs that mostly match as
// "moved with changes". The spike only needs plausible input for the viewers.
import { lineDiff, splitLines } from './diff';
import type { FileDiff, Move } from './model';

const MIN_LINES = 3;
const MIN_CHARS = 20;

interface Line {
  file: number;
  hunk: number;
  line: number; // 1-based
  norm: string;
  used: boolean;
}

const norm = (s: string) => s.trim().replace(/\s+/g, ' ');

export function detectMoves(files: FileDiff[]): Move[] {
  const del: Line[][] = []; // runs of removed lines, one per hunk
  const add: Line[][] = [];
  files.forEach((f, fi) => {
    const a = splitLines(f.original);
    const b = splitLines(f.modified);
    f.hunks.forEach((h, hi) => {
      if (h.oldLines > 0)
        del.push(range(h.oldStart, h.oldLines).map((l) => ({ file: fi, hunk: hi, line: l, norm: norm(a[l - 1] ?? ''), used: false })));
      if (h.newLines > 0)
        add.push(range(h.newStart, h.newLines).map((l) => ({ file: fi, hunk: hi, line: l, norm: norm(b[l - 1] ?? ''), used: false })));
    });
  });

  // Index removed lines by text.
  const index = new Map<string, { run: number; pos: number }[]>();
  del.forEach((run, r) =>
    run.forEach((l, p) => {
      if (l.norm.length < 2) return; // don't anchor on blank lines or lone braces
      const list = index.get(l.norm) ?? [];
      list.push({ run: r, pos: p });
      index.set(l.norm, list);
    }),
  );

  const moves: Move[] = [];
  let id = 1;
  for (const run of add) {
    let i = 0;
    while (i < run.length) {
      let best = { len: 0, run: -1, pos: -1 };
      for (const c of index.get(run[i].norm) ?? []) {
        const d = del[c.run];
        if (d[c.pos].used) continue;
        if (d[c.pos].file === run[i].file && d[c.pos].hunk === run[i].hunk) continue; // an edit, not a move
        let len = 0;
        while (i + len < run.length && c.pos + len < d.length && !d[c.pos + len].used && d[c.pos + len].norm === run[i + len].norm) len++;
        if (len > best.len) best = { len, run: c.run, pos: c.pos };
      }
      const exact = run.slice(i, i + best.len);
      const chars = exact.reduce((s, l) => s + l.norm.length, 0);
      if (best.len >= MIN_LINES && chars >= MIN_CHARS) {
        // Grow the exact anchor over neighbouring lines that were changed in the move,
        // tolerating single mismatched lines. Any mismatch makes it "moved with changes".
        const d = del[best.run];
        const grow = (dir: 1 | -1) => {
          let n = 0;
          let ok = 0;
          let misses = 0;
          let changed = false;
          for (;;) {
            const si = dir > 0 ? best.pos + best.len + n : best.pos - 1 - n;
            const ai = dir > 0 ? i + best.len + n : i - 1 - n;
            if (si < 0 || ai < 0 || si >= d.length || ai >= run.length || d[si].used || run[ai].used) break;
            if (d[si].norm === run[ai].norm) {
              misses = 0;
              ok = n + 1;
            } else if (++misses > 1) break;
            else changed = true;
            n++;
          }
          return { n: ok, changed: changed && ok > 0 };
        };
        const before = grow(-1);
        const after = grow(1);
        const src = d.slice(best.pos - before.n, best.pos + best.len + after.n);
        const block = run.slice(i - before.n, i + best.len + after.n);
        src.forEach((l) => (l.used = true));
        block.forEach((l) => (l.used = true));
        moves.push(toMove(id++, before.changed || after.changed ? 'moved-changed' : 'moved', files, src, block));
        i += best.len + after.n;
      } else i++;
    }
  }

  // Second pass: moved with changes.
  const leftovers = (runs: Line[][]) => runs.flatMap(splitUnused).filter((r) => r.length >= MIN_LINES);
  const delLeft = leftovers(del);
  for (const a of leftovers(add)) {
    let best = { score: 0, idx: -1 };
    delLeft.forEach((d, idx) => {
      if (d[0].used) return;
      if (d[0].file === a[0].file && d[0].hunk === a[0].hunk) return;
      const score = similarity(d, a);
      if (score > best.score) best = { score, idx };
    });
    if (best.score >= 0.5) {
      const d = delLeft[best.idx];
      d.forEach((l) => (l.used = true));
      a.forEach((l) => (l.used = true));
      moves.push(toMove(id++, 'moved-changed', files, d, a));
    }
  }
  return moves.map((m) => ({ ...m, changed: [...changedLinesInMove(m, files)] }));
}

function splitUnused(run: Line[]): Line[][] {
  const out: Line[][] = [];
  let cur: Line[] = [];
  for (const l of run) {
    if (l.used) {
      if (cur.length) out.push(cur);
      cur = [];
    } else cur.push(l);
  }
  if (cur.length) out.push(cur);
  return out;
}

function similarity(a: Line[], b: Line[]): number {
  const counts = new Map<string, number>();
  for (const l of a) if (l.norm.length >= 2) counts.set(l.norm, (counts.get(l.norm) ?? 0) + 1);
  let common = 0;
  for (const l of b) {
    const n = counts.get(l.norm) ?? 0;
    if (n > 0 && l.norm.length >= 2) {
      common++;
      counts.set(l.norm, n - 1);
    }
  }
  return common < MIN_LINES ? 0 : common / Math.max(a.length, b.length);
}

function toMove(id: number, kind: Move['kind'], files: FileDiff[], src: Line[], dst: Line[]): Move {
  return {
    id,
    kind,
    from: { path: files[src[0].file].path, start: src[0].line, end: src[src.length - 1].line },
    to: { path: files[dst[0].file].path, start: dst[0].line, end: dst[dst.length - 1].line },
  };
}

function range(start: number, count: number): number[] {
  return Array.from({ length: count }, (_, i) => start + i);
}

/** Destination lines of a moved-with-changes block that differ from the source block. */
export function changedLinesInMove(move: Move, files: FileDiff[]): Set<number> {
  const out = new Set<number>();
  if (move.kind !== 'moved-changed') return out;
  const src = files.find((f) => f.path === move.from.path);
  const dst = files.find((f) => f.path === move.to.path);
  if (!src || !dst) return out;
  const a = splitLines(src.original).slice(move.from.start - 1, move.from.end);
  const b = splitLines(dst.modified).slice(move.to.start - 1, move.to.end);
  for (const h of lineDiff(a.join('\n'), b.join('\n')))
    for (let i = 0; i < h.newLines; i++) out.add(move.to.start + h.newStart - 1 + i);
  return out;
}
