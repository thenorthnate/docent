# M0 spike: diff/code viewer

_November 2026. Code on the `spike/viewer-choice` branch (never merged). Answers Q2 and 8a in [`OPEN-QUESTIONS.md`](../OPEN-QUESTIONS.md)._

## Recommendation

**CodeMirror 6** (`@codemirror/merge`). Both viewers can do everything M0 asked for. CodeMirror is the only one that shows **git's** diff instead of its own, and Docent's design depends on that: diffs come from local git, `docent-git` detects moves, and findings are anchored to git's lines. It is also lighter and faster on large files. Monaco's advantage is built-in polish (overview ruler, diff navigator, find widget), which Docent would rebuild in CodeMirror where it needs it.

The WebKitGTK load test has not run yet (see below).

## What was tested

The same inputs went to both viewers, in split and unified mode:

- a 3-file PR with a move within one file, a move across files, and a move across files with one line changed
- generated TypeScript files of 5,000, 20,000 and 100,000 lines with an edit every ~120 lines
- 133 modified source files from a real 40-commit range of an open-source PHP/Vue app, with hunks from `git diff -U0`

Timings come from the Tauri app on macOS (WKWebView, 60 Hz display), from two runs that agreed closely. The browser pane was used only to check rendering.

## Findings

### 1. Whose diff is shown (decisive)

| | CodeMirror 6 | Monaco |
|---|---|---|
| Accepts an external diff | Yes: `diffConfig.override` returns the changes; the spike converts git hunks and refines inside each hunk for intra-line highlights | No public API. The diff editor always computes its own (`diffAlgorithm` only picks between built-in ones) |
| Files whose hunks match git exactly (real repo) | 131 / 133 | 91 / 133 |
| Differences | 2: CodeMirror's `makePresentable` pass slid one pure deletion by a line, and merged two hunks separated by one unchanged line | 124 hunk boundaries in 42 files, mostly pure insertions or deletions shifted by one line across a blank line or brace. On the demo PR, Monaco showed 2 hunks where git had 3 |

Why it matters: anything Docent places from git's hunks lands in the wrong place when the viewer disagrees. In the spike, a "moved to …" header in unified view sat one line away from the deleted block it describes. The same would apply to "is this line in the diff" checks and to GitHub comment positions.

The two CodeMirror differences are cosmetic and fixable: wrap the changes so `makePresentable` leaves them alone, or upstream a small option to skip it. Monaco has no equivalent hook short of forking its diff provider.

### 2. Moved blocks within and across files (answers 8a)

Both can render them, with Docent supplying the moves:

- **Split view:** a clickable header above the block ("⇄ Moved from `claims.ts:4–9` · click to jump") and a tint on its lines, on both sides. Monaco uses view zones, and CodeMirror uses block widgets. Jumping to another file works in both. Both keep the two sides aligned when the headers are added.
- **Moved with changes:** the changed lines get a stronger tint. A full inner diff wasn't built; both could show one in a widget.
- **Tint:** CodeMirror's line class replaces the diff colour completely. In Monaco, the tint covers the line background, but Monaco's character-level insert/delete highlight still shows through, and there's no API to switch it off for a range.
- **Unified view:** both can put the header above the deleted block, but neither can tint or mark individual deleted lines, because the viewer owns their DOM.
- Monaco's `experimental.showMoves` is confirmed to work within one file only, using Monaco's own detection. It isn't needed, because `docent-git` detects moves.

### 3. Gutter decorations

Both are fine. Monaco has glyph-margin decorations with hover messages built in, and CodeMirror needs a custom `gutter` of about 30 lines. In unified mode, neither can put a marker on a **deleted** line (both log "nowhere to go"). Findings on removed lines will need split view, or a marker on the nearest new-side line.

### 4. Hover `?` at the far right

Both work, through one shared fixed-position button. Each viewer reports the hovered line's geometry: `getScrolledVisiblePosition` in Monaco, `lineBlockAt` in CodeMirror. Clicking it reports the right file, side and line. Neither offers it on deleted lines in unified view.

### 5. Large-file performance (macOS WKWebView)

Scroll: 400 frames at 120 px per frame. "Ready" is the time from mount until the diff is shown with decorations.

| Lines | Viewer | Ready (split / unified) | Worst frame (split / unified) | Frames > 33 ms | DOM nodes (split / unified) |
|---|---|---|---|---|---|
| 5,014 | CodeMirror | 40 / 31 ms | 27 / 28 ms | 0 | 458 / 229 |
| 5,014 | Monaco | 95 / 81 ms | 18 / 18 ms | 0 | 1,482 / 1,845 |
| 20,047 | CodeMirror | 39 / 35 ms | 26 / 23 ms | 0 | 458 / 228 |
| 20,047 | Monaco | 136 / 123 ms | 19 / 19 ms | 0 | 1,687 / 2,794 |
| 100,257 | CodeMirror | 58 / 55 ms | 36 / 27 ms | 0–1 | 458 / 228 |
| 100,257 | Monaco | 348 / 373 ms | 126 / 127 ms | 1–2 | 2,273 / 7,878 |

Both hold 60 fps at every size. CodeMirror is ready 2–6× sooner and keeps its DOM size flat. At 100k lines, Monaco drops one or two frames of ~125 ms. Small files: CodeMirror 15–19 ms, Monaco 36–46 ms.

### 6. Bundle size (Vite production build)

| | JS | Gzipped JS | Also needs |
|---|---|---|---|
| CodeMirror (merge, view, state, language, TypeScript grammar) | 410 KB | 140 KB | — |
| Monaco, trimmed to the core and contributions plus TypeScript highlighting | 3.94 MB | 1.02 MB | 304 KB worker, 186 KB CSS, 153 KB icon font |

Size matters less in a desktop app, but parse time at start-up does.

### 7. Integration notes

- **Monaco:** needs a web worker set up through Vite (`?worker`). Vite's dev-mode dependency optimiser broke the worker import from the package, so a one-line local shim was needed. Monaco 0.57's default entry pulls in every language and language service; trimming it means copying its contribution list. Disposing the editor inside its own mouse handler (a cross-file jump) left the next editor stuck, so the jump has to be deferred.
- **CodeMirror:** every `@codemirror/*` package must resolve to one copy of `@codemirror/state` and `@codemirror/view`, so pin compatible versions. More chrome is Docent's to build: overview ruler, hunk navigation and search UI. `collapseUnchanged` exists.

### 8. WebKitGTK (Linux) load test: not run yet

It can't run on the Mac. `.github/workflows/spike-webkitgtk.yml` on the spike branch builds the app on Ubuntu 24.04 with WebKitGTK 4.1 and runs the same headless benchmark under Xvfb. The workflow runs when the branch is pushed, and its results become the `webkitgtk-results` artifact. Record them here before closing M0.

## Not covered

- Memory use: WebKit exposes no JS heap API, and the WebContent process wasn't measured.
- Dark theme, accessibility (screen readers, keyboard navigation of the diff) and very long lines.
- Vue integration. Both mount on a plain element, so a thin wrapper component should do.
- Inner diffs for "moved with changes", beyond highlighting the changed lines.
