# M0 viewer spike: Monaco vs CodeMirror 6

Throwaway code for the M0 milestone in [`docs/PLAN.md` §7](../docs/PLAN.md#7-milestones). It lives on `spike/viewer-choice` and is **never merged**. The findings are in [`docs/spikes/viewer-choice.md`](../docs/spikes/viewer-choice.md).

It mounts the same diffs in both viewers and checks the four things M0 asks about: gutter decorations, a hover `?` at the far right of a line, large-file performance, and moved blocks within and across files.

## What's here

| Path | What it is |
|---|---|
| `ui/src/viewers/monaco.ts` | Monaco diff editor: glyph-margin markers, view zones and line decorations for moves, hunk comparison against git |
| `ui/src/viewers/codemirror.ts` | `@codemirror/merge` (split and unified), fed git's hunks through `diffConfig.override`; custom gutter, block widgets for moves |
| `ui/src/viewers/monaco-entry.ts` | Trimmed Monaco import (editor core and contributions, TypeScript highlighting only), so the bundle comparison is fair |
| `ui/src/hover.ts` | The hover `?`, shared by both viewers |
| `ui/src/moveHeader.ts` | The move header (collapse arrow + jump label), shared by both viewers |
| `ui/src/moves.ts` | Moved-block detection standing in for `docent-git` (exact runs of 3+ lines across files, grown into "moved with changes") |
| `ui/src/diff.ts` | Myers line diff standing in for `git diff -U0` on the browser fixtures |
| `ui/src/fixtures.ts` | A 3-file PR with a within-file move, a cross-file move and a cross-file move with changes; generated 5k/20k/100k-line files |
| `ui/src/main.ts` | Shell: viewer/mode/fixture pickers, event log, metrics, benchmark runner |
| `src-tauri/` | Tauri 2 shell: loads real `git diff -U0` hunks from a local repo, saves benchmark JSON, headless benchmark mode |
| `results/` | Benchmark output from macOS WKWebView |
| `../.github/workflows/spike-webkitgtk.yml` | Runs the same headless benchmark on Ubuntu (WebKitGTK 4.1, Xvfb) when this branch is pushed |

## Run it

Needs Node 22, Rust stable and Xcode command-line tools.

```bash
cd spike/ui && npm install
```

**In a browser** (fast to poke at; Chromium, not the target webview):

```bash
npm --prefix spike/ui run dev
```

Open http://localhost:5173. Pick a viewer, mode and fixture. Hover a line for the `?`, click a purple move header to jump to the other end, click its ▸/▾ arrow to collapse or expand the block, and watch the Events panel. `window.runBenchmark()` in the console runs the suite.

**In Tauri on macOS** (WKWebView, the real target). `cargo run` serves the built UI from `ui/dist`, so build it first:

```bash
npm --prefix spike/ui run build && cd spike/src-tauri && cargo run
```

For live reload, start the dev server (above) and run `cargo run --no-default-features`, which loads http://localhost:5173 instead. Without the dev server running, that window is blank.

The **Load git diff** box (Tauri only) loads `git diff -U0 <base> <head>` from any local repo.

**Headless benchmark** (builds the UI, runs every case, writes `results/bench-<os>-<time>.json`, quits). Keep the window visible while it runs: a hidden window pauses `requestAnimationFrame`, and the scroll test is skipped.

```bash
npm --prefix spike/ui run build && cd spike/src-tauri && cargo build --release && SPIKE_BENCH=1 ./target/release/docent-viewer-spike
```

Add `SPIKE_GIT="/path/to/repo|<base>|<head>"` to include a real diff, which checks hunk agreement with git file by file.

**WebKitGTK:** push the branch, then download the `webkitgtk-results` artifact from the `spike-webkitgtk` workflow run.
