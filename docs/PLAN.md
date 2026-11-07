# GAIT — Project Plan (Draft 1)

> **GAIT** = Git + AI. A cross-platform desktop app for AI-guided code review **and** code exploration, which drives whatever coding agent you already use.

**Status:** Draft for review. Anything marked **🟡 Proposed** is my recommendation and waits on your answer in [`OPEN-QUESTIONS.md`](./OPEN-QUESTIONS.md) (referenced as `Q#`). Anything marked **✅ Decided** came from you.

---

## 1. Goals

1. Review GitHub (then GitLab) PRs from a desktop app without opening the browser.
2. Know immediately what changed since **you** last looked at a PR: new commits, comments, review state.
3. Click any line and get an explanation of *why* it is there and what it relates to, grounded in the **whole repository** (local clone), not just the diff.
4. Have a small team of agents (explain, bugs, style consistency, modernization/lint) do the legwork automatically, with no prompting required.
5. Write comments and submit reviews from the app.
6. Explore any branch or commit with the same click-a-line experience, even with no PR.
7. **Agent-agnostic**: Kiro at work, Claude Code at home, Codex/Copilot/Gemini/anything ACP-capable.

### Non-goals (v1)

- Not a code editor or IDE. GAIT never edits your working copy. (Suggested fixes become review comments or suggestions.)
- No hosted backend or GAIT server. Everything runs locally; GAIT talks only to your forge and to locally spawned agents.
- No model API keys inside GAIT (✅ Decided: ACP agents only). Billing and auth stay with each agent.

---

## 2. Decisions so far

| Area | Decision | Status |
|---|---|---|
| App framework | Tauri 2 (Rust backend, system webview) | ✅ Decided |
| Frontend | Vue 3 + TypeScript + Vite | ✅ Decided (Tauri 2 ships an official Vue template) |
| Styling | Tailwind CSS v4 | ✅ Decided |
| Agent integration | ACP (Agent Client Protocol) only; GAIT is an ACP **client** | ✅ Decided |
| Forges | GitHub first, GitLab second, behind one provider interface | ✅ Decided |
| Audience | Open source (MIT, already in repo) | ✅ Decided |
| Local repos | Use your existing clones; GAIT creates a git **worktree** per PR so your working copy is never touched | ✅ Decided |
| UI component library | shadcn-vue (Reka UI + Tailwind) | ✅ Decided |
| Diff/code viewer | Prototype Monaco **and** CodeMirror 6 in M0, pick after | ✅ Decided (via spike) |
| Agent auto-run policy | Review lenses auto-run when you open a PR at a new head SHA (on-demand above a size threshold); Explain on click | ✅ Decided |
| Agent permissions | Agents read-only, no commands; GAIT itself detects and runs the repo's linters and feeds results to agents | ✅ Decided |

---

## 3. Architecture overview

```
┌──────────────────────────── Tauri 2 app ────────────────────────────┐
│  Vue 3 UI (webview)                                                  │
│   Inbox · PR view · Diff viewer · Explore view · Agent side panel    │
│        ▲ invoke() commands / events (tauri::ipc)                     │
│  ──────┼───────────────────────────────────────────────────────────  │
│  Rust core                                                           │
│   ┌────────────┐ ┌──────────────┐ ┌────────────┐ ┌───────────────┐   │
│   │ gait-forge │ │  gait-git    │ │ gait-agents│ │  gait-store   │   │
│   │ GitHub /   │ │ worktrees,   │ │ ACP client │ │ SQLite: state,│   │
│   │ GitLab API │ │ diffs, blame │ │ manager,   │ │ review marks, │   │
│   │ + poller   │ │ (git CLI)    │ │ lenses     │ │ findings cache│   │
│   └─────┬──────┘ └──────┬───────┘ └─────┬──────┘ └───────────────┘   │
└─────────┼───────────────┼───────────────┼────────────────────────────┘
          │HTTPS          │local          │stdio JSON-RPC (ACP)
     GitHub / GitLab   your clones    kiro-cli acp · claude-agent-acp ·
                       + worktrees    codex-acp · copilot --acp · …
```

### 3.1 Repository layout (Cargo workspace + pnpm)

```
gait/
├─ src-tauri/            # Tauri app shell: commands, events, window setup, plugins
├─ crates/
│  ├─ gait-core/         # shared domain types (Repo, PullRequest, Finding, Lens…)
│  ├─ gait-forge/        # Forge trait + github/ + gitlab/ impls, poller
│  ├─ gait-git/          # worktree mgmt, diff/interdiff, blame, file reads
│  ├─ gait-agents/       # ACP client, agent registry, lens runner, output parsing
│  └─ gait-store/        # SQLite schema + migrations + queries
├─ ui/                   # Vue 3 + TS + Vite + Tailwind
│  ├─ src/views/         # Inbox, PullRequest, Explore, Settings
│  ├─ src/components/    # DiffViewer, FileTree, AgentPanel, FindingCard, CommentBox…
│  └─ src/stores/        # Pinia stores
├─ lenses/               # built-in agent definitions (markdown + frontmatter)
├─ docs/
└─ .github/workflows/    # CI + release builds (tauri-action)
```

Splitting the Rust side into small crates keeps the forge, git and agent logic testable without Tauri, and lets a future CLI reuse them.

### 3.2 Off-the-shelf building blocks

| Need | Choice | Why |
|---|---|---|
| Scaffold | `create-tauri-app` (Vue + TS template) | Official, zero custom glue |
| UI state / routing | Pinia, Vue Router | Vue standard |
| Components | shadcn-vue on Reka UI | Tailwind-native, copy-in components, accessible |
| Diff & code view | Monaco **or** CodeMirror 6 (`@codemirror/merge`), chosen in M0 spike | Monaco: VS Code-grade diff; CodeMirror: lighter. Both support gutter decorations |
| Markdown in panels | `markdown-it` + Shiki for code blocks | Agent output is markdown |
| GitHub API | `octocrab` (REST + GraphQL) | Mature Rust client |
| GitLab API | `gitlab` crate, or `reqwest` + typed models | Evaluate in M7 |
| Git operations | System `git` CLI via `tokio::process` | Worktrees, range-diff and blame are first-class in git; avoids reimplementing |
| ACP | `agent-client-protocol` crate | Official Rust SDK |
| Persistence | SQLite via `sqlx` (migrations) | Local, zero-config |
| Secrets | `keyring` crate (macOS Keychain / Windows Credential Manager / Secret Service) | Never store tokens in plaintext |
| OS notifications | `tauri-plugin-notification` | Official plugin |
| Auto-update | `tauri-plugin-updater` | Official plugin, works with GitHub Releases |
| Settings files | `tauri-plugin-store` or TOML in app config dir | Simple |
| Logging | `tracing` + `tauri-plugin-log` | Debuggable agent sessions |

---

## 4. Core concepts

- **Repo binding**: a forge repo (e.g. `acme/api` on github.com) mapped to a local clone path. GAIT discovers clones by scanning folders you choose and matching `git remote` URLs. You can also add them by hand.
- **Workspace**: a git worktree GAIT creates under its own data dir (e.g. `~/.gait/worktrees/<repo>/<pr-number>`), checked out at the PR's head SHA. Agents run with `cwd` = workspace, so they see the full repo at exactly the reviewed commit, and your own checkout is untouched.
- **Agent profile**: how to launch an ACP agent (command, args, env), e.g. `kiro-cli acp`, `npx @agentclientprotocol/claude-agent-acp`, `codex-acp`, `copilot --acp`. Ships with presets and auto-detection on `PATH`. Profiles can be scoped per repo (e.g. work repos → Kiro, personal → Claude Code).
- **Lens**: a reusable review job: a prompt template + expected output schema + which agent profile runs it + when it runs. The four default lenses are below. Lenses are plain markdown files with frontmatter, so they're easy to edit, share, and override per repo (`.gait/lenses/*.md`, 🟡 Q7).
- **Finding**: structured lens output pinned to `file:line(s)` with severity, category, rationale, optional suggested replacement, and the evidence the agent used. Findings are **private to you** until you promote one into a review comment (🟡 Q5).
- **Review state**: per-PR record of the last head SHA you reviewed, which files you marked viewed, and which comments/events you've seen. This drives "changed since last review".

---

## 5. Features in detail

### 5.1 Accounts and connection (M1)

- **GitHub**: OAuth **device flow** (no client secret needed, works for an open-source desktop app) with a registered GAIT OAuth App (🟡 Q9). Personal access token as fallback. GitHub Enterprise Server via custom base URL.
- **GitLab** (M7): personal access token first; OAuth (PKCE) for gitlab.com and self-managed later.
- Multiple accounts at once, e.g. work GitHub Enterprise + personal github.com.
- Tokens are kept in the OS keychain.

### 5.2 Inbox (M1)

- Sections: **Review requested**, **Assigned**, **Authored by me**, **Mentioned**, **Watching (pinned)**.
- Per row: title, repo, author, CI status, review state, size, age, and an **"updated since your last review"** badge with counts (`+3 commits · 5 comments`).
- Filters and saved views. Keyboard navigation (j/k, enter).
- Data comes from GitHub GraphQL search queries (`review-requested:@me`, `assignee:@me`, …) batched into one request per poll.

### 5.3 Change tracking and notifications (M5)

- **Poller** (Rust, background): polls each account at an interval (default 60s, 🟡 Q10) using conditional requests (ETag) to stay inside rate limits. Webhooks aren't possible for a desktop app without a server.
- On each poll it compares against stored review state: new head SHA, new comments/reviews, new review requests, CI state changes.
- **"Since last review" diff**: GAIT stores the head SHA you last reviewed. When new commits arrive it shows an **interdiff** (`git diff <lastReviewed>..<newHead>`). If history was rewritten (force-push/rebase), it falls back to `git range-diff` so you see what actually changed rather than the whole PR again.
- New and unseen comments are highlighted. "Mark all seen" is available.
- OS notifications with per-event-type toggles. Clicking one opens the PR.
- Sync with GitHub's own **"Viewed" file checkboxes** (GraphQL `markFileAsViewed`), so progress shows on the web too.

### 5.4 PR review view (M1–M4)

Layout:

```
┌ File tree (status, viewed ✓, finding counts) ┬ Diff (Monaco) ─────────────┬ Agent panel ─────────┐
│                                               │  gutter: ● findings         │ [Explain] [Bugs]     │
│                                               │          💬 comments        │ [Style] [Modernize]  │
│                                               │  click line → panel updates │ [Chat]               │
└───────────────────────────────────────────────┴─────────────────────────────┴──────────────────────┘
 Top bar: PR title · branch → base · CI · "3 new commits since your review" · [Submit review ▾]
```

- Overview tab: description, checks, timeline, and an AI summary produced by the explainer lens.
- Diff modes: unified or split, full PR or since-last-review, ignore-whitespace.
- Show/hide findings by lens and severity.
- Clicking a line or selecting a range sets the "focus" and every lens panel updates for that location.

### 5.5 Agents ("lenses") (M2–M3)

All lenses run through ACP sessions with `cwd` = the PR's worktree, so the agent can read anything in the repo using its own tools (search, read, LSP, etc.).

| Lens | Trigger | Input GAIT provides | Output |
|---|---|---|---|
| **Explain** | On click (cached per `file:line@sha`), plus a PR-level summary on open | PR title/body, linked issues, the hunk, `git blame` / log for the surrounding lines, related-changes hints | Markdown: *what* the line does, *why* it's there in this PR, **related changes** (other files/lines in the PR that go with it, as clickable links), callers/impact |
| **Bugs** | Automatically when you open a PR at a new head SHA | Full diff + PR context | JSON findings: correctness, edge cases, security, concurrency, error handling |
| **Style** | Automatically (same rule) | Diff + instruction to compare with conventions elsewhere in the repo; contributor docs such as `CONTRIBUTING.md`, `AGENTS.md`, `CLAUDE.md`, `.editorconfig` | JSON findings citing the in-repo example it compared against |
| **Modernize & lint** | Automatically (same rule) | Diff + language/framework versions detected from the repo + **linter output GAIT ran itself** | JSON findings: simpler or newer idioms, deprecated APIs, linter violations, each with a suggested replacement |
| **Chat** | Manual | Current focus (file/line/selection) + prior panel content | Free-form follow-up conversation in the same ACP session |

**How "related changes" works**: GAIT pre-computes cheap structural hints before asking the agent, such as other hunks touching the same symbols (ripgrep + optional tree-sitter), co-changed files, and test files. These go into the prompt and the agent confirms or expands them. That keeps answers fast and grounded.

**Structured output**: each lens prompt ends with a required JSON block (schema in the lens frontmatter). GAIT validates it (`serde_json` + `jsonschema`). On failure it retries once with the validation error, and if it still fails it shows the raw markdown. ACP doesn't define structured output, so this keeps us agent-neutral.

**Caching and cost control**: results are cached in SQLite keyed by `(lens, repo, head_sha, scope)`, so reopening a PR is free. There's a concurrency limit per agent profile, a cancel button (`session/cancel`), and a per-PR "run agents" toggle.

**Agent profiles and mapping**: Settings → Agents lists detected agents and their status (installed / signed in). Each lens can pick a profile (e.g. Bugs → Claude Code, Explain → Kiro). One profile can be the default for all lenses.

### 5.6 Agent safety and permissions (M2)

GAIT is the ACP **client**, so it answers every agent permission request:

- **Allow**: read/search/list files inside the worktree.
- **Deny**: file writes and edits, git pushes, network-mutating tools. GAIT doesn't advertise `fs/write_text_file`.
- **Commands**: always denied to agents. Instead GAIT detects the repo's linters (`eslint`, `ruff`, `cargo clippy`, `golangci-lint`, …) and runs them itself on changed files in the worktree, then passes the output to the Modernize lens.
- The worktree is disposable, so even a misbehaving agent can't touch your real checkout.
- Every session transcript (prompts, tool calls, permission decisions) is viewable in an "Agent activity" drawer for transparency and debugging.

### 5.7 Commenting and submitting reviews (M4)

- Click the gutter or select lines → comment box (markdown, preview, GitHub `suggestion` blocks).
- **Promote a finding** to a comment in one click. GAIT pre-fills the text and you edit it before posting.
- Comments collect into a **pending review** (GitHub's draft review API, GitLab draft notes), then go out together with **Comment / Approve / Request changes**.
- Reply to, resolve and unresolve existing threads. Single comments can also post immediately.
- Optional: "draft with agent". You write the gist and the explainer lens polishes it into a clear review comment, never posting automatically.

### 5.8 Explore mode (M6)

- Pick any bound repo, then a branch, tag or commit (default: the default branch). GAIT creates or refreshes a worktree at that ref.
- File tree and code viewer (Monaco read-only), with the same click-a-line agent panel:
  - **Explain** uses blame/log to answer why the code exists ("introduced in #1234 to fix …").
  - **Bugs / Style / Modernize** run on demand for the current file or selection rather than automatically.
- **Automatic mode switching**: if the selected branch has an open PR, GAIT offers to jump into Review mode for it.
- What you can do with explore findings (🟡 Q6): keep them as private notes, copy them as markdown, or open a GitHub/GitLab **issue** pre-filled with the finding.

### 5.9 GitLab (M7)

- Implement the `Forge` trait for GitLab: MR inbox (`reviewer_username`, `assignee`), MR diffs and versions (GitLab exposes diff versions natively, which helps since-last-review), discussions, draft notes, approvals.
- Self-managed GitLab via base URL.

---

## 6. Data model (SQLite, initial)

```
accounts(id, forge, base_url, login, keychain_ref)
repos(id, account_id, owner, name, default_branch, local_path)
pull_requests(id, repo_id, number, title, author, state, head_sha, base_sha, updated_at, raw_json)
review_state(pr_id, last_reviewed_sha, last_seen_event_at, viewed_files_json)
events(id, pr_id, kind, actor, created_at, seen BOOL)          -- commits, comments, reviews, CI
agent_profiles(id, name, command, args_json, env_json, scope)
lens_runs(id, lens, pr_id NULL, repo_id, ref_sha, scope, status, started_at, finished_at, transcript_path)
findings(id, lens_run_id, file, line_start, line_end, severity, category, title, body_md, suggestion, dismissed BOOL)
pending_comments(id, pr_id, file, line, side, body_md, from_finding_id NULL)
notes(id, repo_id, ref_sha, file, line, body_md)               -- explore mode
```

---

## 7. Milestones

Each milestone is a set of short-lived branches off `main`, so independent work can run in parallel.

| # | Milestone | Outcome | Can parallelize with |
|---|---|---|---|
| **M0** | **Spikes** (de-risk) | (a) Rust ACP client drives `kiro-cli acp` and `claude-agent-acp`: initialize → session in a worktree → prompt → stream → permission prompts. (b) Tauri 2 + Vue + Tailwind + shadcn-vue scaffold with CI building on all 3 OSes. (c) Monaco **vs** CodeMirror 6 diff in a Tauri webview (gutter decorations, line-click events, large-file performance, Linux WebKitGTK) → pick one. | a, b, c are independent |
| **M1** | GitHub inbox + read-only PR view | Device-flow login, repo↔clone binding, inbox, PR overview, diff viewer, worktree per PR | — |
| **M2** | Explain lens + agent settings | Agent profiles and detection, permission policy, Explain on click, PR summary, Chat, activity drawer | M4 UI work |
| **M3** | Review lenses | Bugs, Style, Modernize & lint; findings in gutter and file tree; caching; lens files | M5 |
| **M4** | Commenting and reviews | Pending review, promote finding, submit/approve/request changes, threads | M2/M3 |
| **M5** | Change tracking | Poller, review state, interdiff/range-diff, badges, OS notifications, viewed-file sync | M3 |
| **M6** | Explore mode | Any-ref worktrees, file browser, on-demand lenses, notes, auto-switch to Review | M7 |
| **M7** | GitLab | Forge impl, MR inbox/diffs/discussions/draft notes | M6 |
| **M8** | Release 0.1 | Signed builds (macOS notarized, Windows signed, Linux AppImage/deb/rpm), auto-update, docs site, contributor guide | — |

**MVP cut (personal use)**: M0–M4. **First public release**: M0–M8.

---

## 8. Testing strategy

- **Rust**: unit tests per crate. `gait-git` tests run against temp repos built in-test. `gait-forge` uses recorded HTTP fixtures (`wiremock`).
- **Mock ACP agent**: a tiny test binary that speaks ACP and replays scripted responses (including permission requests and malformed JSON), so the lens pipeline is tested without real agents or cost.
- **Frontend**: Vitest + Vue Test Utils for components and stores. Playwright for a few smoke flows against the Vite dev build with mocked `invoke`.
- **CI**: GitHub Actions matrix (macOS arm64/x64, Ubuntu, Windows) running `cargo fmt/clippy/test`, `pnpm lint/test`, and `tauri build` (artifacts on tags via `tauri-action`).

## 9. Open-source hygiene

- MIT license (already present). Add `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, issue/PR templates, `SECURITY.md` (token handling, agent sandboxing).
- `AGENTS.md` / `CLAUDE.md` describing repo conventions so AI agents (and GAIT itself) can work on GAIT.
- Conventional commits + `release-please` for changelogs and versioning.
- No telemetry by default.

## 10. Risks and mitigations

| Risk | Mitigation |
|---|---|
| ACP agents differ in capabilities and output quality | Validate the JSON schema with a retry, per-agent quirk flags in profiles, mock-agent tests, M0 spike against Kiro + Claude first |
| Agent cost and latency on big PRs | Cache by SHA, run lenses per-file in parallel with a cap, run on demand for huge PRs (size threshold), cancel |
| Diff viewer bundle size / webview differences (WebKitGTK on Linux) | Decided by M0 spike (c) |
| GitHub rate limits from polling | GraphQL batching, ETags, backoff, adaptive interval |
| Auth friction in enterprises (SSO, GHES, self-managed GitLab) | PAT fallback, custom base URLs |
| Agents attempting writes | Client-side permission policy, no write capability advertised, disposable worktrees |
| Force-pushes break "since last review" | `git range-diff` fallback; keep old SHAs fetchable via `refs/gait/*` |
