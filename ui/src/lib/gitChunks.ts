/**
 * Shows git's hunks in CodeMirror's unified merge view exactly (PLAN §5.4).
 *
 * `diffConfig.override` alone can't: CodeMirror's `makePresentable` pass can
 * slide a pure insertion or deletion by a line, and its chunking merges
 * changes one short or blank line apart, whatever the override returns. So
 * for Docent's documents `Chunk.build` is replaced, and the chunks are built
 * from git's hunks directly. Upstreaming an option to skip those passes is
 * the long-term fix.
 */
import { Chunk, presentableDiff, unifiedMergeView, type DiffConfig } from "@codemirror/merge";
import { EditorState, Text, type Extension } from "@codemirror/state";
import type { Hunk } from "@/lib/review";

/** Each Docent document's hunks, keyed by the diff config its view was given. */
const gitHunks = new WeakMap<DiffConfig, readonly Hunk[]>();

const libraryBuild = Chunk.build.bind(Chunk);
Chunk.build = (a, b, conf) => {
  const hunks = conf && gitHunks.get(conf);
  return hunks ? chunksOf(a, b, hunks) : libraryBuild(a, b, conf);
};

/**
 * A side of a hunk as document positions: from the start of its first line
 * to one past the end of its last, which may be past the end of the
 * document, as `Chunk` has it. A side with no lines is the point after line
 * `start`.
 */
function span(doc: Text, start: number, lines: number): [number, number] {
  if (lines === 0) {
    const at = start < doc.lines ? doc.line(start + 1).from : doc.length;
    return [at, at];
  }
  return [doc.line(start).from, doc.line(start + lines - 1).to + 1];
}

function chunksOf(a: Text, b: Text, hunks: readonly Hunk[]): Chunk[] {
  return hunks.map((hunk) => {
    const [fromA, toA] = span(a, hunk.old_start, hunk.old_lines);
    const [fromB, toB] = span(b, hunk.new_start, hunk.new_lines);
    // Only the highlights inside the hunk are CodeMirror's own.
    const inner = presentableDiff(a.sliceString(fromA, toA), b.sliceString(fromB, toB), { scanLimit: 500 });
    return new Chunk(inner, fromA, toA, fromB, toB);
  });
}

/**
 * A read-only unified view of the editor's document against `old`, showing
 * git's `hunks` between them. Lines split on `\n` only, as in git, so the
 * editor must be given its document as is.
 */
export function gitDiffView(old: string, hunks: readonly Hunk[]): Extension {
  const diffConfig: DiffConfig = {};
  gitHunks.set(diffConfig, hunks);
  return [
    EditorState.lineSeparator.of("\n"),
    EditorState.readOnly.of(true),
    unifiedMergeView({ original: Text.of(old.split("\n")), diffConfig, mergeControls: false }),
  ];
}
