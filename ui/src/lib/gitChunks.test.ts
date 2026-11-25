import { describe, expect, it } from "vitest";
import { getChunks } from "@codemirror/merge";
import { EditorState } from "@codemirror/state";
import { gitDiffView } from "@/lib/gitChunks";
import type { Hunk } from "@/lib/review";

/** The hunks CodeMirror shows for `old` → `now`, given git's `hunks`, in git's own terms. */
function shownHunks(old: string, now: string, hunks: Hunk[]): Hunk[] {
  const state = EditorState.create({ doc: now, extensions: gitDiffView(old, hunks) });
  const a = EditorState.create({ doc: old, extensions: EditorState.lineSeparator.of("\n") }).doc;
  const side = (doc: typeof a, from: number, to: number) => {
    const start = doc.lineAt(from).number;
    return to === from ? [start - 1, 0] : [start, doc.lineAt(to - 1).number - start + 1];
  };
  return getChunks(state)!.chunks.map((chunk) => {
    const [old_start, old_lines] = side(a, chunk.fromA, chunk.toA);
    const [new_start, new_lines] = side(state.doc, chunk.fromB, chunk.toB);
    return { old_start, old_lines, new_start, new_lines };
  });
}

const hunk = (old_start: number, old_lines: number, new_start: number, new_lines: number): Hunk => ({
  old_start,
  old_lines,
  new_start,
  new_lines,
});

describe("gitDiffView", () => {
  // The M0 spike's cases, with git's own `diff -U0` hunks for them.
  it("keeps a pure deletion next to a blank line where git put it", () => {
    const hunks = [hunk(2, 2, 1, 0), hunk(8, 1, 6, 1), hunk(10, 1, 8, 1)];

    const shown = shownHunks("x\nfoo\n\nfoo\nbar\n\n}\nkeep\n}\nlast\n", "x\nfoo\nbar\n\n}\nnew\n}\nLAST\n", hunks);

    expect(shown).toEqual(hunks);
  });

  it("keeps two hunks one unchanged line apart separate", () => {
    const hunks = [hunk(1, 1, 1, 1), hunk(3, 1, 3, 1)];

    expect(shownHunks("a\n\nc\n", "A\n\nC\n", hunks)).toEqual(hunks);
    expect(shownHunks("a\n}\nc\n", "A\n}\nC\n", hunks)).toEqual(hunks);
  });

  it("shows whole added and deleted files, and files without a final newline", () => {
    expect(shownHunks("", "one\ntwo", [hunk(0, 0, 1, 2)])).toEqual([hunk(0, 0, 1, 2)]);
    expect(shownHunks("a\nb\n", "", [hunk(1, 2, 0, 0)])).toEqual([hunk(1, 2, 0, 0)]);
    expect(shownHunks("a\nb", "a\nc", [hunk(2, 1, 2, 1)])).toEqual([hunk(2, 1, 2, 1)]);
  });

  it("counts lines as git does: a lone carriage return doesn't end one", () => {
    const hunks = [hunk(2, 1, 2, 1)];

    expect(shownHunks("a\rb\nc\n", "a\rb\nC\n", hunks)).toEqual(hunks);
  });
});
