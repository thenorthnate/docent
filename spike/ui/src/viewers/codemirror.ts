// CodeMirror 6 adapter: @codemirror/merge (split or unified), fed git's hunks via
// diffConfig.override, with a custom gutter and block widgets for moves.
import { Change, MergeView, getChunks, diff as cmDiff, unifiedMergeView } from '@codemirror/merge';
import { EditorState, RangeSet, Text, StateEffect, StateField, type Extension, type Range as CmRange } from '@codemirror/state';
import { Decoration, EditorView, GutterMarker, WidgetType, gutter, highlightSpecialChars, lineNumbers, type DecorationSet } from '@codemirror/view';
import { defaultHighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { javascript } from '@codemirror/lang-javascript';
import { newLineBefore } from '../diff';
import { HoverQuestion } from '../hover';
import { jumpTarget, moveHeader, startsCollapsed } from '../moveHeader';
import { compareHunks, frameStats, nextFrame, settle, type Annotation, type FileDiff, type Hunk, type MountOptions, type MountStats, type Move, type ScrollStats, type Side, type Viewer } from '../model';
import { hunkKeyOf } from './hunks';

export class CodeMirrorViewer implements Viewer {
  readonly name = 'codemirror' as const;
  private merge?: MergeView;
  private unified?: EditorView;
  private hover?: HoverQuestion;
  private scroller?: HTMLElement;
  private onScroll = () => this.hover?.hide();

  async mount(el: HTMLElement, file: FileDiff, opts: MountOptions): Promise<MountStats> {
    const t0 = performance.now();
    this.hover = new HoverQuestion((side, line) => opts.onExplain(file.path, side, line));
    const diffConfig = { override: (a: string, b: string) => changesFromHunks(a, b, file.hunks) };
    const unified = opts.mode === 'unified';

    if (unified) {
      const old = opts.annotations.filter((a) => a.path === file.path && a.side === 'old');
      if (old.length) opts.log(`codemirror/unified: ${old.length} old-side marker(s) have nowhere to go (deleted lines are widgets)`);
      this.unified = new EditorView({
        parent: el,
        doc: file.modified,
        extensions: [
          this.common(file, 'new', opts),
          unifiedMergeView({ original: file.original, diffConfig, mergeControls: false, gutter: true, highlightChanges: true, syntaxHighlightDeletions: true }),
        ],
      });
      this.scroller = this.unified.scrollDOM;
    } else {
      this.merge = new MergeView({
        parent: el,
        a: { doc: file.original, extensions: this.common(file, 'old', opts) },
        b: { doc: file.modified, extensions: this.common(file, 'new', opts) },
        diffConfig,
        gutter: true,
        highlightChanges: true,
      });
      this.scroller = this.merge.dom;
    }
    this.scroller.addEventListener('scroll', this.onScroll, { passive: true });
    const createMs = performance.now() - t0;
    await settle();

    // Compare the chunks CodeMirror shows with the git hunks.
    const state = (this.unified ?? this.merge!.b).state;
    const docA = this.merge ? this.merge.a.state.doc : Text.of(file.original.split('\n'));
    const docB = state.doc;
    const chunks = getChunks(state)?.chunks ?? [];
    const viewerKeys = chunks.map((c) => {
      const oldLines = c.fromA === c.toA ? 0 : docA.lineAt(c.endA).number - docA.lineAt(c.fromA).number + 1;
      const newLines = c.fromB === c.toB ? 0 : docB.lineAt(c.endB).number - docB.lineAt(c.fromB).number + 1;
      const oldStart = docA.lineAt(c.fromA).number - (oldLines ? 0 : 1);
      const newStart = docB.lineAt(c.fromB).number - (newLines ? 0 : 1);
      return hunkKeyOf({ oldStart, oldLines, newStart, newLines });
    });
    const gitKeys = file.hunks.map(hunkKeyOf);
    return { createMs, readyMs: performance.now() - t0, ...compareHunks(viewerKeys, gitKeys), viewerHunks: chunks.length };
  }

  private common(file: FileDiff, side: Side, opts: MountOptions): Extension {
    const anns = opts.annotations.filter((a) => a.path === file.path && a.side === side);
    const unified = opts.mode === 'unified';
    const hover = this.hover!;
    return [
      lineNumbers(),
      annotationGutter(anns),
      highlightSpecialChars(),
      syntaxHighlighting(defaultHighlightStyle),
      javascript({ typescript: true }),
      EditorState.readOnly.of(true),
      moveDecorations(file, side, opts.moves, unified, opts.onJump),
      flashField,
      EditorView.theme({ '&': { fontSize: '13px' }, '.cm-content': { lineHeight: '19px' } }),
      EditorView.domEventHandlers({
        mousemove(e, view) {
          const lineEl = (e.target as Element).closest('.cm-line');
          if (!lineEl || (e.target as Element).closest('.cm-deletedChunk')) return hover.hide();
          const pos = view.posAtCoords({ x: e.clientX, y: e.clientY }, false);
          const block = view.lineBlockAt(pos);
          const rect = view.dom.getBoundingClientRect();
          const sb = view.scrollDOM.offsetWidth - view.scrollDOM.clientWidth;
          hover.show({ side, line: view.state.doc.lineAt(pos).number, top: view.documentTop + block.top, height: Math.min(block.height, 19), right: rect.right - sb });
        },
        mouseleave(e) {
          hover.hide(e);
        },
      }),
    ];
  }

  reveal(side: Side, line: number) {
    const view = side === 'old' && this.merge ? this.merge.a : (this.merge?.b ?? this.unified);
    if (!view) return;
    const pos = view.state.doc.line(Math.min(line, view.state.doc.lines)).from;
    view.dispatch({ effects: [EditorView.scrollIntoView(pos, { y: 'center' }), flash.of(pos)] });
    setTimeout(() => view.dispatch({ effects: flash.of(-1) }), 1200);
  }

  async scrollBench(px: number, maxFrames: number): Promise<ScrollStats> {
    const s = this.scroller!;
    s.scrollTop = 0;
    await nextFrame();
    const deltas: number[] = [];
    let last = await nextFrame();
    for (let i = 0; i < maxFrames && s.scrollTop + s.clientHeight < s.scrollHeight - 1; i++) {
      s.scrollTop += px;
      const now = await nextFrame();
      deltas.push(now - last);
      last = now;
    }
    return frameStats(deltas);
  }

  dispose() {
    this.scroller?.removeEventListener('scroll', this.onScroll);
    this.hover?.dispose();
    this.merge?.destroy();
    this.unified?.destroy();
    this.merge = this.unified = undefined;
  }
}

/** git line hunks → CodeMirror char Changes, refined inside each hunk for intra-line highlights. */
export function changesFromHunks(a: string, b: string, hunks: Hunk[]): Change[] {
  const startsA = lineStarts(a);
  const startsB = lineStarts(b);
  const at = (starts: number[], text: string, line: number) => (line - 1 < starts.length ? starts[line - 1] : text.length);
  const out: Change[] = [];
  for (const h of hunks) {
    const fromA = at(startsA, a, h.oldLines ? h.oldStart : h.oldStart + 1);
    const toA = at(startsA, a, (h.oldLines ? h.oldStart : h.oldStart + 1) + h.oldLines);
    const fromB = at(startsB, b, h.newLines ? h.newStart : h.newStart + 1);
    const toB = at(startsB, b, (h.newLines ? h.newStart : h.newStart + 1) + h.newLines);
    if (fromA === toA || fromB === toB) {
      out.push(new Change(fromA, toA, fromB, toB));
      continue;
    }
    for (const c of cmDiff(a.slice(fromA, toA), b.slice(fromB, toB), { scanLimit: 500 }))
      out.push(new Change(c.fromA + fromA, c.toA + fromA, c.fromB + fromB, c.toB + fromB));
  }
  return out;
}

function lineStarts(text: string): number[] {
  const out = [0];
  for (let i = 0; i < text.length; i++) if (text.charCodeAt(i) === 10 && i + 1 < text.length) out.push(i + 1);
  return out;
}

// --- Gutter markers -------------------------------------------------------

class AnnMarker extends GutterMarker {
  constructor(readonly anns: Annotation[]) {
    super();
  }
  toDOM() {
    const wrap = document.createElement('span');
    for (const a of this.anns) {
      const s = document.createElement('span');
      s.className = a.kind === 'comment' ? 'gm gm-comment' : `gm gm-finding gm-${a.severity ?? 'medium'}`;
      s.title = a.text;
      wrap.appendChild(s);
    }
    return wrap;
  }
}

function annotationGutter(anns: Annotation[]): Extension {
  let cached: RangeSet<GutterMarker> | null = null;
  return gutter({
    class: 'cm-ann-gutter',
    markers: (view) => {
      if (cached) return cached;
      const byLine = new Map<number, Annotation[]>();
      for (const a of anns) if (a.line <= view.state.doc.lines) byLine.set(a.line, [...(byLine.get(a.line) ?? []), a]);
      const ranges = [...byLine.entries()].sort((x, y) => x[0] - y[0]).map(([l, list]) => new AnnMarker(list).range(view.state.doc.line(l).from));
      return (cached = RangeSet.of(ranges));
    },
    initialSpacer: () => new AnnMarker([{ path: '', side: 'new', line: 0, kind: 'finding', text: '' }]),
  });
}

// --- Moves -----------------------------------------------------------------

const toggleMove = StateEffect.define<number>();

class MoveHeader extends WidgetType {
  constructor(
    readonly move: Move,
    readonly end: Side,
    readonly collapsible: boolean,
    readonly collapsed: boolean,
    readonly onJump: MountOptions['onJump'],
  ) {
    super();
  }
  eq(o: MoveHeader) {
    return o.move.id === this.move.id && o.end === this.end && o.collapsed === this.collapsed;
  }
  toDOM(view: EditorView) {
    const r = this.end === 'new' ? this.move.to : this.move.from;
    const t = jumpTarget(this.move, this.end);
    return moveHeader(this.move, this.end, r.end - r.start + 1, this.collapsible, this.collapsed, {
      onToggle: () => view.dispatch({ effects: toggleMove.of(this.move.id) }),
      onJump: () => this.onJump(t.path, t.side, t.line),
    }).dom;
  }
  ignoreEvent() {
    return true;
  }
}

/**
 * Header widget + tint for each move end in this editor. A collapsed block is hidden with
 * a block replace decoration (the mechanism collapseUnchanged uses, so MergeView keeps
 * both sides aligned). Deleted lines in unified view belong to the merge extension and
 * can't be hidden, so those headers have no arrow.
 */
function moveDecorations(file: FileDiff, side: Side, moves: Move[], unified: boolean, onJump: MountOptions['onJump']): Extension {
  const collapsedField = StateField.define<ReadonlySet<number>>({
    create: () => new Set(moves.filter(startsCollapsed).map((m) => m.id)),
    update(v, tr) {
      for (const e of tr.effects)
        if (e.is(toggleMove)) {
          const next = new Set(v);
          if (!next.delete(e.value)) next.add(e.value);
          v = next;
        }
      return v;
    },
  });
  const build = (state: EditorState): DecorationSet => {
    const doc = state.doc;
    const collapsed = state.field(collapsedField);
    const decos: CmRange<Decoration>[] = [];
    const line = (n: number) => doc.line(Math.max(1, Math.min(n, doc.lines)));
    const header = (m: Move, end: Side, collapsible: boolean, pos: number, s: -1 | 1) =>
      decos.push(Decoration.widget({ widget: new MoveHeader(m, end, collapsible, collapsible && collapsed.has(m.id), onJump), block: true, side: s }).range(pos));
    const block = (m: Move, start: number, end: number, cls: (l: number) => string) => {
      if (collapsed.has(m.id)) decos.push(Decoration.replace({ block: true }).range(line(start).from, line(end).to));
      else for (let l = start; l <= end; l++) decos.push(Decoration.line({ class: cls(l) }).range(line(l).from));
    };
    for (const m of moves) {
      if (side === 'new' && m.to.path === file.path) {
        const changed = new Set(m.changed ?? []);
        header(m, 'new', true, line(m.to.start).from, -1);
        block(m, m.to.start, m.to.end, (l) => (changed.has(l) ? 'mv-line mv-line-changed' : 'mv-line'));
      }
      if (m.from.path !== file.path) continue;
      if (unified && side === 'new') {
        const after = newLineBefore(file.hunks, m.from.start);
        header(m, 'old', false, after >= doc.lines ? doc.length : line(after + 1).from, after >= doc.lines ? 1 : -1);
      } else if (!unified && side === 'old') {
        header(m, 'old', true, line(m.from.start).from, -1);
        block(m, m.from.start, m.from.end, () => 'mv-line-away');
      }
    }
    return Decoration.set(decos, true);
  };
  const decoField = StateField.define<DecorationSet>({
    create: build,
    update: (v, tr) => (tr.effects.some((e) => e.is(toggleMove)) ? build(tr.state) : v),
    provide: (f) => EditorView.decorations.from(f),
  });
  return [collapsedField, decoField];
}

// --- Jump flash --------------------------------------------------------------

const flash = StateEffect.define<number>();
const flashField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(v, tr) {
    for (const e of tr.effects) if (e.is(flash)) v = e.value < 0 ? Decoration.none : Decoration.set([Decoration.line({ class: 'flash-line' }).range(e.value)]);
    return v;
  },
  provide: (f) => EditorView.decorations.from(f),
});
