// The header shown above a moved block, shared by both viewers:
// [▸/▾ arrow: collapse or expand] [label: jump to the other end of the move].
import type { Move, Side } from './model';

/** Exact moves start collapsed; moves with changes stay open because they need review. */
export const startsCollapsed = (m: Move) => m.kind === 'moved';

export function moveLabel(m: Move, end: Side): string {
  const other = end === 'new' ? m.from : m.to; // header on the new side points back to the source
  const verb = end === 'new' ? (m.kind === 'moved' ? 'Moved from' : 'Moved with changes from') : 'Moved to';
  return `${verb} ${other.path}:${other.start}–${other.end}`;
}

export interface HeaderDom {
  dom: HTMLElement;
  setCollapsed(collapsed: boolean): void;
}

export interface HeaderHandlers {
  onToggle(): void;
  onJump(): void;
}

/**
 * `collapsible` is false where the viewer owns the lines (deleted lines in unified view).
 * The arrow toggles; anything else on the header jumps to the other end of the move.
 */
export function moveHeader(m: Move, end: Side, lines: number, collapsible: boolean, collapsed: boolean, h: HeaderHandlers): HeaderDom {
  const dom = document.createElement('div');
  dom.className = `mv-zone mv-${m.kind}`;
  const arrow = document.createElement('span');
  arrow.className = 'mv-arrow';
  const label = document.createElement('span');
  label.className = 'mv-label';
  const note = document.createElement('span');
  note.className = 'mv-note';
  dom.append(arrow, label, note);
  label.textContent = `⇄ ${moveLabel(m, end)}`;
  const setCollapsed = (c: boolean) => {
    arrow.textContent = collapsible ? (c ? '▸' : '▾') : '';
    arrow.title = c ? 'Show moved lines' : 'Hide moved lines';
    note.textContent = collapsible && c ? ` · ${lines} line${lines === 1 ? '' : 's'} hidden` : '';
  };
  setCollapsed(collapsed);
  // Handled on mousedown and stopped here, so the editor underneath never sees the click.
  dom.addEventListener('mousedown', (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (collapsible && e.target instanceof Element && e.target.closest('.mv-arrow')) h.onToggle();
    else h.onJump();
  });
  return { dom, setCollapsed };
}

/** The other end of a move, as a jump target, seen from the header at `end`. */
export function jumpTarget(m: Move, end: Side): { side: Side; path: string; line: number } {
  return end === 'new' ? { side: 'old', path: m.from.path, line: m.from.start } : { side: 'new', path: m.to.path, line: m.to.start };
}
