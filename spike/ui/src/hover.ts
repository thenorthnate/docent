// The hover `?` at the far right of a line (PLAN §5.4). One fixed-position button
// shared by both viewers; each adapter reports the hovered line's screen rect.
import type { Side } from './model';

export interface HoverTarget {
  side: Side;
  line: number;
  top: number; // viewport px
  height: number;
  right: number; // viewport px of the text area's right edge
}

export class HoverQuestion {
  readonly el: HTMLButtonElement;
  private target: HoverTarget | null = null;

  constructor(onClick: (side: Side, line: number) => void) {
    this.el = document.createElement('button');
    this.el.className = 'hover-q';
    this.el.textContent = '?';
    this.el.title = 'Explain this line';
    this.el.addEventListener('mousedown', (e) => e.preventDefault()); // keep editor focus/selection
    this.el.addEventListener('click', () => this.target && onClick(this.target.side, this.target.line));
    document.body.appendChild(this.el);
  }

  show(t: HoverTarget) {
    this.target = t;
    const size = Math.min(18, t.height);
    Object.assign(this.el.style, {
      display: 'block',
      top: `${t.top + (t.height - size) / 2}px`,
      left: `${t.right - size - 6}px`,
      width: `${size}px`,
      height: `${size}px`,
    });
  }

  /** Hide unless the pointer moved onto the button itself. */
  hide(e?: MouseEvent) {
    if (e && e.relatedTarget === this.el) return;
    this.el.style.display = 'none';
    this.target = null;
  }

  dispose() {
    this.el.remove();
  }
}
