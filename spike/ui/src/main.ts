import './style.css';
import { fixtureFromGit, largeFixture, movesDemo } from './fixtures';
import { lineCount, round, type FileDiff, type Fixture, type MountStats, type ScrollStats, type Side, type Viewer } from './model';

type ViewerName = 'monaco' | 'codemirror';
type Mode = 'split' | 'unified';

const tauri = '__TAURI_INTERNALS__' in window;
async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const core = await import('@tauri-apps/api/core');
  return core.invoke<T>(cmd, args);
}

// Viewers load lazily so each lands in its own chunk (bundle size is part of the comparison).
async function createViewer(name: ViewerName): Promise<Viewer> {
  if (name === 'monaco') return new (await import('./viewers/monaco')).MonacoViewer();
  return new (await import('./viewers/codemirror')).CodeMirrorViewer();
}

const $ = <T extends HTMLElement>(sel: string) => document.querySelector(sel) as T;
const viewerEl = $('#viewer');
const logEl = $('#log');
const metricsEl = $('#metrics');

const state = {
  viewer: 'codemirror' as ViewerName,
  mode: 'split' as Mode,
  fixture: movesDemo() as Fixture,
  file: 0,
  current: null as Viewer | null,
  busy: false,
  lastBench: '',
};

function log(msg: string) {
  const li = document.createElement('li');
  li.textContent = `${new Date().toLocaleTimeString()}  ${msg}`;
  logEl.prepend(li);
  if (tauri) void invoke('log_line', { line: msg });
}

// --- Top bar -----------------------------------------------------------------

const fixtures: Record<string, () => Fixture> = {
  moves: movesDemo,
  l5k: () => largeFixture(5_000),
  l20k: () => largeFixture(20_000),
  l100k: () => largeFixture(100_000),
};

$('#bar').innerHTML = `
  <strong>Viewer spike</strong>
  <label>Viewer <select id="viewer-sel"><option value="codemirror">CodeMirror 6</option><option value="monaco">Monaco</option></select></label>
  <label>Mode <select id="mode-sel"><option value="split">Split</option><option value="unified">Unified</option></select></label>
  <label>Fixture <select id="fixture-sel">
    <option value="moves">Moves demo (3 files)</option>
    <option value="l5k">Large: 5,000 lines</option>
    <option value="l20k">Large: 20,000 lines</option>
    <option value="l100k">Large: 100,000 lines</option>
  </select></label>
  <span id="git" ${tauri ? '' : 'hidden'}>
    <input id="git-repo" placeholder="/path/to/repo" size="22" />
    <input id="git-base" placeholder="base (e.g. main~20)" size="12" />
    <input id="git-head" placeholder="head" size="8" value="HEAD" />
    <button id="git-load">Load git diff</button>
  </span>
  <button id="bench">Run benchmark</button>
  <span class="ua">${tauri ? 'Tauri' : 'Browser'} · ${navigator.userAgent.match(/(AppleWebKit|Chrome|Firefox|Version)\/[\d.]+/g)?.join(' ') ?? ''}</span>`;

$<HTMLSelectElement>('#viewer-sel').onchange = (e) => {
  state.viewer = (e.target as HTMLSelectElement).value as ViewerName;
  void show();
};
$<HTMLSelectElement>('#mode-sel').onchange = (e) => {
  state.mode = (e.target as HTMLSelectElement).value as Mode;
  void show();
};
$<HTMLSelectElement>('#fixture-sel').onchange = (e) => {
  state.fixture = fixtures[(e.target as HTMLSelectElement).value]();
  state.file = 0;
  renderFiles();
  void show();
};
$('#git-load').onclick = async () => {
  const repo = $<HTMLInputElement>('#git-repo').value.trim();
  const base = $<HTMLInputElement>('#git-base').value.trim();
  const head = $<HTMLInputElement>('#git-head').value.trim() || 'HEAD';
  try {
    const files = await invoke<FileDiff[]>('load_git_diff', { repo, base, head });
    state.fixture = fixtureFromGit(`${repo} ${base}..${head}`, files);
    state.file = 0;
    log(`Loaded ${files.length} files from git, ${state.fixture.moves.length} moves detected`);
    renderFiles();
    void show();
  } catch (err) {
    log(`git load failed: ${err}`);
  }
};
$('#bench').onclick = () => void runBenchmark();

// --- File list ----------------------------------------------------------------

function renderFiles() {
  const nav = $('#files');
  nav.innerHTML = `<h3>${state.fixture.name}</h3>`;
  state.fixture.files.forEach((f, i) => {
    const movesIn = state.fixture.moves.filter((m) => m.to.path === f.path).length;
    const movesOut = state.fixture.moves.filter((m) => m.from.path === f.path).length;
    const b = document.createElement('button');
    b.className = i === state.file ? 'file active' : 'file';
    b.innerHTML = `<span>${f.path}</span><small>${f.hunks.length} hunks · ${lineCount(f.modified).toLocaleString()} lines${movesIn + movesOut ? ` · ⇄ ${movesIn} in / ${movesOut} out` : ''}</small>`;
    b.onclick = () => {
      state.file = i;
      renderFiles();
      void show();
    };
    nav.appendChild(b);
  });
  if (state.fixture.moves.length) {
    const ul = document.createElement('ul');
    ul.className = 'moves';
    for (const m of state.fixture.moves)
      ul.insertAdjacentHTML('beforeend', `<li>${m.kind}: ${m.from.path}:${m.from.start}–${m.from.end} → ${m.to.path}:${m.to.start}–${m.to.end}</li>`);
    nav.append(Object.assign(document.createElement('h3'), { textContent: 'Detected moves' }), ul);
  }
}

// --- Mounting -------------------------------------------------------------------

async function mountCurrent(): Promise<{ viewer: Viewer; stats: MountStats }> {
  state.current?.dispose();
  viewerEl.innerHTML = '';
  const host = document.createElement('div');
  host.className = 'host';
  viewerEl.appendChild(host);
  const viewer = await createViewer(state.viewer);
  state.current = viewer;
  const file = state.fixture.files[state.file];
  const stats = await viewer.mount(host, file, {
    mode: state.mode,
    annotations: state.fixture.annotations,
    moves: state.fixture.moves,
    log,
    onExplain: (path, side, line) => log(`? Explain → ${path}:${line} (${side})`),
    // Deferred: a cross-file jump disposes the viewer, which must not happen inside its own event handler.
    onJump: (path, side, line) => setTimeout(() => void jump(path, side, line)),
  });
  return { viewer, stats };
}

async function show() {
  if (state.busy) return;
  const file = state.fixture.files[state.file];
  try {
    const { stats } = await mountCurrent();
    metricsEl.textContent = describe(file, stats) + (state.lastBench ? `\n\nLast benchmark\n${state.lastBench}` : '');
  } catch (err) {
    log(`mount failed: ${err}`);
    console.error(err);
  }
}

async function jump(path: string, side: Side, line: number) {
  log(`jump → ${path}:${line} (${side})`);
  const idx = state.fixture.files.findIndex((f) => f.path === path);
  if (idx < 0) return log(`  ${path} is not in this diff`);
  if (idx !== state.file) {
    state.file = idx;
    renderFiles();
    await show();
  }
  state.current?.reveal(side, line);
}

function describe(file: FileDiff, s: MountStats, scroll?: ScrollStats) {
  return [
    `${state.viewer} · ${state.mode} · ${file.path}`,
    `create ${round(s.createMs)} ms · ready ${round(s.readyMs)} ms`,
    `viewer hunks ${s.viewerHunks} / git ${file.hunks.length} · mismatches ${s.hunkMismatches} ${s.mismatchDetail.join(' ')}`,
    `DOM nodes in viewer: ${viewerEl.querySelectorAll('*').length.toLocaleString()}`,
    scroll ? `scroll: ${scroll.frames} frames, mean ${scroll.meanMs} ms, p95 ${scroll.p95Ms} ms, max ${scroll.maxMs} ms, >33ms ${scroll.longFrames}` : '',
  ].join('\n');
}

// --- Benchmark -----------------------------------------------------------------

interface BenchRow {
  viewer: ViewerName;
  mode: Mode;
  fixture: string;
  file: string;
  lines: number;
  createMs: number;
  readyMs: number;
  viewerHunks: number;
  gitHunks: number;
  hunkMismatches: number;
  mismatchDetail: string[];
  domNodes: number;
  scroll?: ScrollStats;
}

async function runBenchmark() {
  if (state.busy) return;
  state.busy = true;
  const saved = { viewer: state.viewer, mode: state.mode, fixture: state.fixture, file: state.file };
  const rows: BenchRow[] = [];
  const cases: { fixture: Fixture; file: number; scroll: boolean; splitOnly?: boolean }[] = [];
  const demo = movesDemo();
  demo.files.forEach((_, i) => cases.push({ fixture: demo, file: i, scroll: false }));
  for (const n of [5_000, 20_000, 100_000]) cases.push({ fixture: largeFixture(n), file: 0, scroll: true });
  // Optional real diff (Tauri only): hunk agreement on git's own output, split mode.
  const target = tauri ? await invoke<string[] | null>('bench_git_target') : null;
  if (target) {
    try {
      const [repo, base, head] = target;
      // Skip generated/minified files: they say nothing about review-sized source diffs.
      const files = (await invoke<FileDiff[]>('load_git_diff', { repo, base, head })).filter(
        (f) => f.original && f.modified && f.original !== f.modified && f.modified.length < 512 * 1024 && !/\.min\.|\/(dist|build|vendor)\//.test(f.path),
      );
      const fx = fixtureFromGit(`git ${base}..${head}`, files);
      log(`benchmark: ${files.length} modified files from git, ${fx.moves.length} moves`);
      files.forEach((_, i) => cases.push({ fixture: fx, file: i, scroll: false, splitOnly: true }));
    } catch (err) {
      log(`benchmark: git load failed: ${err}`);
    }
  }
  log('benchmark started');
  try {
    // Warm-up mount for each viewer so chunk loading and worker start-up aren't timed.
    for (const v of ['monaco', 'codemirror'] as ViewerName[]) {
      Object.assign(state, { viewer: v, mode: 'split', fixture: demo, file: 0 });
      await mountCurrent();
    }
    for (const c of cases)
      for (const v of ['monaco', 'codemirror'] as ViewerName[])
        for (const mode of (c.splitOnly ? ['split'] : ['split', 'unified']) as Mode[]) {
          Object.assign(state, { viewer: v, mode, fixture: c.fixture, file: c.file });
          const f = c.fixture.files[c.file];
          let mounted: Awaited<ReturnType<typeof mountCurrent>>;
          try {
            mounted = await mountCurrent();
          } catch (err) {
            log(`bench ${v}/${mode} ${f.path}: mount failed: ${err}`);
            continue;
          }
          const { viewer, stats } = mounted;
          await new Promise((r) => setTimeout(r, 300));
          // rAF is paused in hidden pages, so frame timing is skipped there.
          const scroll = c.scroll && !document.hidden ? await viewer.scrollBench(120, 400) : undefined;
          const row: BenchRow = {
            viewer: v,
            mode,
            fixture: c.fixture.name,
            file: f.path,
            lines: lineCount(f.modified),
            createMs: round(stats.createMs),
            readyMs: round(stats.readyMs),
            viewerHunks: stats.viewerHunks,
            gitHunks: f.hunks.length,
            hunkMismatches: stats.hunkMismatches,
            mismatchDetail: stats.mismatchDetail,
            domNodes: viewerEl.querySelectorAll('*').length,
            scroll,
          };
          rows.push(row);
          metricsEl.textContent = describe(f, stats, scroll);
          log(`bench ${v}/${mode} ${f.path}: ready ${row.readyMs} ms${scroll ? `, scroll p95 ${scroll.p95Ms} ms` : ''}`);
        }
  } finally {
    state.busy = false;
  }
  const report = { userAgent: navigator.userAgent, tauri, date: new Date().toISOString(), rows };
  state.lastBench = table(rows);
  console.log(JSON.stringify(report, null, 2));
  if (tauri) {
    const path = await invoke<string>('save_results', { json: JSON.stringify(report, null, 2) });
    log(`results saved to ${path}`);
  }
  Object.assign(state, saved);
  renderFiles();
  await show();
  return report;
}

function table(rows: BenchRow[]): string {
  const head = 'viewer      mode     lines   ready ms  hunks(v/git/miss)    dom   scroll mean/p95/max ms  >33ms file  mismatches';
  return [
    head,
    ...rows.map((r) =>
      [
        r.viewer.padEnd(11),
        r.mode.padEnd(8),
        String(r.lines).padStart(6),
        String(r.readyMs).padStart(9),
        `${r.viewerHunks}/${r.gitHunks}/${r.hunkMismatches}`.padStart(18),
        String(r.domNodes).padStart(6),
        (r.scroll ? `${r.scroll.meanMs}/${r.scroll.p95Ms}/${r.scroll.maxMs}` : '-').padStart(22),
        String(r.scroll?.longFrames ?? '-').padStart(6),
        r.file.split('/').pop(),
        r.mismatchDetail.join(' '),
      ].join(' '),
    ),
  ].join('\n');
}

// --- Start -------------------------------------------------------------------------

renderFiles();
await show();
if (tauri && (await invoke<boolean>('bench_requested'))) {
  try {
    await runBenchmark();
  } catch (err) {
    log(`benchmark failed: ${err}`);
  } finally {
    await invoke('quit');
  }
}
(window as unknown as { runBenchmark: typeof runBenchmark }).runBenchmark = runBenchmark;
