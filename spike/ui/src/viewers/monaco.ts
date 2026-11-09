// Monaco adapter: diff editor + glyph-margin markers + view zones for moves.
import * as monaco from './monaco-entry';
import EditorWorker from './monaco.worker?worker';
import { newLineBefore } from '../diff';
import { HoverQuestion } from '../hover';
import { jumpTarget, moveHeader, startsCollapsed, type HeaderDom } from '../moveHeader';
import { hunkKeyOf } from './hunks';
import { compareHunks, frameStats, nextFrame, type FileDiff, type MountOptions, type MountStats, type Move, type ScrollStats, type Side, type Viewer } from '../model';

(self as unknown as { MonacoEnvironment: monaco.Environment }).MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};

type Editor = monaco.editor.ICodeEditor;

export class MonacoViewer implements Viewer {
  readonly name = 'monaco' as const;
  private diff?: monaco.editor.IStandaloneDiffEditor;
  private models: monaco.editor.ITextModel[] = [];
  private hover?: HoverQuestion;
  private disposables: monaco.IDisposable[] = [];

  async mount(el: HTMLElement, file: FileDiff, opts: MountOptions): Promise<MountStats> {
    const t0 = performance.now();
    const unified = opts.mode === 'unified';
    this.diff = monaco.editor.createDiffEditor(el, {
      readOnly: true,
      renderSideBySide: !unified,
      useInlineViewWhenSpaceIsLimited: false,
      glyphMargin: true,
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 13,
      lineHeight: 19,
      scrollBeyondLastLine: false,
      renderOverviewRuler: true,
      // Monaco's own single-file move detection, for comparison with ours.
      experimental: { showMoves: true },
      maxComputationTime: 0, // never give up; we want to see the real cost
      maxFileSize: 0,
    });
    const lang = file.path.endsWith('.ts') ? 'typescript' : undefined;
    const original = monaco.editor.createModel(file.original, lang);
    const modified = monaco.editor.createModel(file.modified, lang);
    this.models = [original, modified];
    // onDidUpdateDiff doesn't fire for every input (e.g. identical texts), so cap the wait.
    let timedOut = false;
    const ready = new Promise<void>((resolve) => {
      const d = this.diff!.onDidUpdateDiff(() => {
        d.dispose();
        resolve();
      });
      setTimeout(() => {
        timedOut = true;
        d.dispose();
        resolve();
      }, 5000);
    });
    this.diff.setModel({ original, modified });
    const createMs = performance.now() - t0;
    await ready;
    if (timedOut) opts.log(`monaco: no diff update within 5 s for ${file.path}`);

    const orig = this.diff.getOriginalEditor();
    const mod = this.diff.getModifiedEditor();
    const editorFor = (side: Side): Editor | null => (side === 'new' ? mod : unified ? null : orig);

    // Gutter markers.
    for (const side of ['old', 'new'] as Side[]) {
      const ed = editorFor(side);
      const anns = opts.annotations.filter((a) => a.path === file.path && a.side === side);
      if (!ed) {
        if (anns.length) opts.log(`monaco/unified: ${anns.length} old-side marker(s) have nowhere to go (original editor hidden)`);
        continue;
      }
      ed.createDecorationsCollection(
        anns.map((a) => ({
          range: new monaco.Range(a.line, 1, a.line, 1),
          options: {
            glyphMarginClassName: a.kind === 'comment' ? 'mgm gm-comment' : `mgm gm-finding gm-${a.severity ?? 'medium'}`,
            glyphMarginHoverMessage: { value: a.text },
          },
        })),
      );
    }

    // Moves: tinted lines + a header zone above each block. The header's arrow hides
    // the block's lines (setHiddenAreas, which works but isn't in Monaco's public typings).
    type Header = { move: Move; ed: Editor; end: Side; collapsible: boolean; header: HeaderDom; toggle(): void; jump(): void };
    const zones = new Map<string, Header>();
    const collapsed = new Set(opts.moves.filter(startsCollapsed).map((m) => m.id));
    const addZone = (ed: Editor, afterLine: number, move: Move, end: Side, collapsible: boolean) => {
      const r = end === 'new' ? move.to : move.from;
      const t = jumpTarget(move, end);
      const toggle = () => {
        if (!collapsed.delete(move.id)) collapsed.add(move.id);
        applyHidden();
      };
      const jump = () => opts.onJump(t.path, t.side, t.line);
      const header = moveHeader(move, end, r.end - r.start + 1, collapsible, collapsible && collapsed.has(move.id), { onToggle: toggle, onJump: jump });
      ed.changeViewZones((acc) => {
        const id = acc.addZone({ afterLineNumber: afterLine, heightInLines: 1, domNode: header.dom, showInHiddenAreas: true });
        zones.set(id, { move, ed, end, collapsible, header, toggle, jump });
      });
    };
    const applyHidden = () => {
      for (const ed of [orig, mod]) {
        const ranges = [...zones.values()]
          .filter((z) => z.ed === ed && z.collapsible && collapsed.has(z.move.id))
          .map((z) => (z.end === 'new' ? z.move.to : z.move.from))
          .map((r) => new monaco.Range(r.start, 1, r.end, 1));
        (ed as unknown as { setHiddenAreas(r: monaco.IRange[], source?: unknown): void }).setHiddenAreas(ranges, 'docent-moves');
      }
      for (const z of zones.values()) z.header.setCollapsed(z.collapsible && collapsed.has(z.move.id));
    };
    for (const m of opts.moves) {
      if (m.to.path === file.path) {
        const changed = new Set(m.changed ?? []);
        mod.createDecorationsCollection(
          range(m.to.start, m.to.end).map((l) => ({
            range: new monaco.Range(l, 1, l, 1),
            options: { isWholeLine: true, className: changed.has(l) ? 'mv-line mv-line-changed' : 'mv-line', linesDecorationsClassName: 'mv-bar', zIndex: 20 },
          })),
        );
        addZone(mod, m.to.start - 1, m, 'new', true);
      }
      if (m.from.path === file.path) {
        if (unified) {
          // Deleted lines are the diff editor's own view zones: they can't be hidden.
          addZone(mod, newLineBefore(file.hunks, m.from.start), m, 'old', false);
        } else {
          orig.createDecorationsCollection(
            range(m.from.start, m.from.end).map((l) => ({
              range: new monaco.Range(l, 1, l, 1),
              options: { isWholeLine: true, className: 'mv-line-away', linesDecorationsClassName: 'mv-bar', zIndex: 20 },
            })),
          );
          addZone(orig, m.from.start - 1, m, 'old', true);
        }
      }
    }
    applyHidden();

    // Hover `?` and zone clicks.
    this.hover = new HoverQuestion((side, line) => opts.onExplain(file.path, side, line));
    for (const [ed, side] of [[orig, 'old'], [mod, 'new']] as [Editor, Side][]) {
      if (side === 'old' && unified) continue;
      this.disposables.push(
        ed.onMouseMove((e) => {
          const t = e.target;
          if (t.type !== monaco.editor.MouseTargetType.CONTENT_TEXT && t.type !== monaco.editor.MouseTargetType.CONTENT_EMPTY) return this.hover!.hide();
          const line = t.position!.lineNumber;
          const vis = ed.getScrolledVisiblePosition({ lineNumber: line, column: 1 });
          if (!vis) return this.hover!.hide();
          const rect = ed.getDomNode()!.getBoundingClientRect();
          const layout = ed.getLayoutInfo();
          this.hover!.show({ side, line, top: rect.top + vis.top, height: vis.height, right: rect.left + layout.width - layout.verticalScrollbarWidth });
        }),
        ed.onMouseLeave((e) => this.hover!.hide(e.event.browserEvent)),
        ed.onDidScrollChange(() => this.hover!.hide()),
        // Monaco's text layer sits above view zones, so the header's own DOM listeners never
        // fire. Route clicks through the editor and split arrow vs label by x position.
        ed.onMouseDown((e) => {
          if (e.target.type !== monaco.editor.MouseTargetType.CONTENT_VIEW_ZONE) return;
          const z = zones.get(e.target.detail.viewZoneId);
          if (!z) return;
          const arrow = z.header.dom.querySelector('.mv-arrow')!.getBoundingClientRect();
          if (z.collapsible && e.event.browserEvent.clientX <= arrow.right + 4) z.toggle();
          else z.jump();
        }),
      );
    }

    // Compare Monaco's own hunks with the git hunks it was given.
    const changes = this.diff.getLineChanges() ?? [];
    const viewerKeys = changes.map((c) =>
      hunkKeyOf({
        oldStart: c.originalStartLineNumber,
        oldLines: c.originalEndLineNumber ? c.originalEndLineNumber - c.originalStartLineNumber + 1 : 0,
        newStart: c.modifiedStartLineNumber,
        newLines: c.modifiedEndLineNumber ? c.modifiedEndLineNumber - c.modifiedStartLineNumber + 1 : 0,
      }),
    );
    const gitKeys = file.hunks.map(hunkKeyOf);

    return { createMs, readyMs: performance.now() - t0, ...compareHunks(viewerKeys, gitKeys), viewerHunks: changes.length };
  }

  reveal(side: Side, line: number) {
    if (!this.diff) return;
    const ed = side === 'old' && this.diff.getOriginalEditor().getDomNode()?.offsetParent ? this.diff.getOriginalEditor() : this.diff.getModifiedEditor();
    ed.revealLineInCenter(line);
    const flash = ed.createDecorationsCollection([{ range: new monaco.Range(line, 1, line, 1), options: { isWholeLine: true, className: 'flash-line' } }]);
    setTimeout(() => flash.clear(), 1200);
  }

  async scrollBench(px: number, maxFrames: number): Promise<ScrollStats> {
    const ed = this.diff!.getModifiedEditor();
    ed.setScrollTop(0);
    await nextFrame();
    const deltas: number[] = [];
    let last = await nextFrame();
    for (let i = 0; i < maxFrames && ed.getScrollTop() + ed.getLayoutInfo().height < ed.getScrollHeight(); i++) {
      ed.setScrollTop(ed.getScrollTop() + px);
      const now = await nextFrame();
      deltas.push(now - last);
      last = now;
    }
    return frameStats(deltas);
  }

  dispose() {
    this.disposables.forEach((d) => d.dispose());
    this.disposables = [];
    this.hover?.dispose();
    this.diff?.dispose();
    this.models.forEach((m) => m.dispose());
    this.diff = undefined;
  }
}

function range(a: number, b: number): number[] {
  return Array.from({ length: b - a + 1 }, (_, i) => a + i);
}
