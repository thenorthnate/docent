# Docent — Project Plan (Draft 2)

> **Docent**: a guide who explains what you're looking at. A desktop app for AI-guided code review **and** code exploration, which drives whatever coding agent you already use.

**Status:** Agreed design, not yet built. Formerly named GAIT. Decisions and remaining unknowns are logged in [`OPEN-QUESTIONS.md`](./OPEN-QUESTIONS.md), and the evidence behind them is in [`RESEARCH.md`](./RESEARCH.md).

**First target:** one user at work: Apple silicon Mac, GitHub Enterprise Cloud with SSO, Kiro, and Jira through Kiro's MCP server. Public release comes after that works.

---

## 1. Goals

1. Review GitHub PRs from a desktop app without opening the browser (GitLab follows in 0.2).
2. Know immediately what changed since **you** last reviewed a PR: new commits, comments, review state.
3. Click the `?` on any line and get an explanation of *why* it is there and what it relates to, grounded in the **whole repository** (local clone), not just the diff.
4. Have a small team of agents (explain, bugs, style consistency, modernization/lint) do the legwork automatically in repos you trust.
5. Write comments and submit reviews from the app.
6. Explore any branch or commit with the same line-level experience, even with no PR.
7. Trace a call path or data flow in one view, including across related repos (for example a service and its infrastructure repo).
8. **Agent-agnostic**: Kiro at work, Claude Code at home, Codex/Copilot/Gemini/anything ACP-capable.

### Non-goals (v1)

- Not a code editor or IDE. Docent never edits your working tree. (Suggested fixes become review comments or suggestions.)
- No hosted backend or Docent server. Everything runs locally; Docent talks only to your forge, locally spawned agents, and GitHub Releases for updates.
- No model API keys inside Docent. Billing and auth stay with each agent.
- No Jira credentials inside Docent. Jira access goes through the agent's own MCP server.

---

## 2. Decisions

| Area | Decision |
|---|---|
| App framework | Tauri 2 (Rust backend, system webview) |
| Frontend | Vue 3 + TypeScript + Vite, Tailwind CSS v4, shadcn-vue |
| Diff/code viewer | Monaco vs CodeMirror 6, chosen by the M0 spike |
| Agent integration | ACP client behind an `AgentBackend` interface, so the way Docent talks to agents can change |
| Forges | GitHub first (`Forge` interface); GitLab in 0.2 |
| Forge auth | Token from `gh auth token --hostname <host>`, held in memory. PAT in the keychain as fallback. Device flow at M8 |
| Issue tracking | `IssueTracker` interface, separate from `Forge`. Jira first, through the agent's MCP server |
| Local repos | Your existing clones, or a normal full clone made by Docent; one git worktree per PR |
| Inbox scope | Watched repos only |
| Hostile content | PR content is hostile until you trust the repo, branch or PR |
| Agent permissions | Best-effort read-only, with an explicit MCP tool allowlist per profile |
| Agent profiles | Per forge account with per-repo overrides; no global fallback |
| Auto-run | Trusted, open, non-draft PRs that request your review or that you pinned, up to a size limit |
| Findings | Private. No auto-post in v1 |
| Audience | Open source (MIT), no telemetry by default |
| Identifiers | `com.thenorthnate.docent`, crates `docent-*`, repo config in `.docent/` |

---

## 3. Architecture overview

```
┌──────────────────────────── Tauri 2 app ─────────────────────────────┐
│  Vue 3 UI (webview)                                                   │
│   Inbox · PR view · Diff viewer · Explore view · Agent side panel     │
│        ▲ invoke() commands / events (tauri::ipc)                      │
│  ──────┼────────────────────────────────────────────────────────────  │
│  Rust core                                                            │
│   ┌──────────────┐ ┌─────────────┐ ┌──────────────┐ ┌──────────────┐  │
│   │ docent-forge │ │ docent-git  │ │ docent-agents│ │ docent-store │  │
│   │ GitHub API   │ │ worktrees,  │ │ AgentBackend,│ │ SQLite: state│  │
│   │ + poller     │ │ diffs, blame│ │ ACP, lenses  │ │ trust, cache │  │
│   └─────┬────────┘ └─────┬───────┘ └─────┬────────┘ └──────────────┘  │
└─────────┼────────────────┼───────────────┼────────────────────────────┘
          │HTTPS           │local          │stdio JSON-RPC (ACP)
     GitHub            your clones     kiro-cli acp · claude-agent-acp ·
                       + worktrees     codex-acp · copilot --acp · …
                                            │ agent's own MCP servers
                                            ▼
                                          Jira
```

### 3.1 Repository layout (Cargo workspace + pnpm)

```
docent/
├─ src-tauri/            # Tauri app shell: commands, events, window setup, plugins
├─ crates/
│  ├─ docent-core/       # shared domain types (Repo, PullRequest, Finding, Lens…)
│  ├─ docent-forge/      # Forge trait + github/ impl + test fake, poller
│  ├─ docent-git/        # worktree mgmt, diff/interdiff, blame, file reads
│  ├─ docent-agents/     # AgentBackend trait, ACP impl, profiles, lens runner, output parsing
│  └─ docent-store/      # SQLite schema + migrations + queries
├─ ui/                   # Vue 3 + TS + Vite + Tailwind
│  ├─ src/views/         # Inbox, PullRequest, Explore, Settings
│  ├─ src/components/    # DiffViewer, FileTree, AgentPanel, FindingCard, CommentBox…
│  └─ src/stores/        # Pinia stores
├─ lenses/               # built-in agent definitions (markdown + frontmatter)
├─ docs/
└─ .github/workflows/    # CI + release builds (tauri-action)
```

Splitting the Rust side into small crates keeps the forge, git and agent logic testable without Tauri, and lets a future CLI reuse them.

### 3.2 Interfaces that keep options open

- **`AgentBackend`**: start session (with one or more roots and a tool policy), stream a prompt, cancel. ACP is the only implementation for now. If Kiro can't run over ACP at work, only this module changes.
- **`Forge`**: GitHub now, GitLab in 0.2. A test fake keeps it from being shaped only around GitHub.
- **`IssueTracker`**: read and write tickets. The first implementation goes through an agent's MCP tools; direct Jira REST can be added later.

### 3.3 Off-the-shelf building blocks

| Need | Choice | Why |
|---|---|---|
| Scaffold | `create-tauri-app` (Vue + TS template) | Official, zero custom glue |
| UI state / routing | Pinia, Vue Router | Vue standard |
| Components | shadcn-vue on Reka UI | Tailwind-native, copy-in components, accessible |
| Diff & code view | Monaco **or** CodeMirror 6 (`@codemirror/merge`), chosen in M0 | Monaco: VS Code-grade diff; CodeMirror: lighter |
| Markdown in panels | `markdown-it` + Shiki for code blocks | Agent output is markdown |
| GitHub API | `octocrab` (REST + GraphQL) | Mature Rust client |
| Git operations | System `git` CLI via `tokio::process` | Worktrees, range-diff and blame are first-class in git |
| ACP | `agent-client-protocol` crate | Official Rust SDK |
| Persistence | SQLite via `sqlx` (migrations) | Local, zero-config |
| Secrets | `keyring` crate | Never store tokens in plaintext |
| OS notifications | `tauri-plugin-notification` | Official plugin |
| Auto-update | `tauri-plugin-updater` | Official plugin, has its own signing key |
| Settings files | `tauri-plugin-store` or TOML in app config dir | Simple |
| Logging | `tracing` + `tauri-plugin-log` | Debuggable agent sessions |

Apps launched from Finder don't inherit your shell's `PATH`, so Docent must load the login shell's environment to find `gh`, `git` and `kiro-cli`.

---

## 4. Core concepts

- **Watched repo**: a forge repo you've told Docent to pay attention to. Everything else stays out of the inbox and notifications.
- **Repo binding**: a watched repo mapped to a local clone. **Bind** points at an existing clone; **Clone** makes a normal full clone into a folder you pick. Diffs always come from local git.
- **Worktree**: a git worktree Docent creates under its own data dir, in detached HEAD at the PR's head SHA. Agents run with `cwd` set to it, so they see the full repo at the reviewed commit. Your working tree is untouched, though your clone's `.git` gains `refs/docent/*` refs and worktree metadata.
- **Trust**: your local decision that a repo, branch or PR may be given to agents and linters (see 5.6).
- **Agent profile**: how to launch an ACP agent (command, args, env), plus its MCP tool allowlist. Assigned per forge account, with optional per-repo overrides. If nothing matches, no agent runs and Docent asks you to choose.
- **Lens**: a reusable review job: a prompt template, an expected output schema, and when it runs. Lenses are markdown files with frontmatter.
- **Finding**: structured lens output pinned to `repo:file:line(s)` with severity, category, rationale, optional suggested replacement, and evidence. Private until you promote it to a comment.
- **Review state**: per-PR record of the last head SHA you reviewed, which files you marked viewed, and which comments/events you've seen.
- **Project**: a named set of bound repos, each at its own ref, explored together (see 5.9).

---

## 5. Features in detail

### 5.1 Accounts and connection

- **MVP**: Docent runs `gh auth token --hostname <host>` at startup and holds the token in memory. If `gh` already reaches your SSO org, no OAuth app registration or org approval is needed. API calls are made directly from Rust.
- **Fallback**: a personal access token stored in the OS keychain.
- **M8**: OAuth device flow for github.com users who don't have `gh`. Each GitHub Enterprise Server instance would need its own registered app, so GHES uses `gh`, a PAT, or a user-supplied client ID.
- Multiple accounts at once, e.g. work and personal.

### 5.2 Inbox (M1)

- Sections: **Review requested**, **Assigned**, **Authored by me**, **Mentioned**, **Pinned**, all limited to watched repos.
- One quiet line for what's filtered out, e.g. "2 review requests in unwatched repos", with a **Watch** button.
- A watched repo with no local clone shows **Bind** and **Clone**.
- Per row: title, repo, author, CI status, review state, size, age, and an **"updated since your last review"** badge with counts (`+3 commits · 5 comments`).
- Filters and saved views. Keyboard navigation (j/k, enter).

### 5.3 Change tracking

**In the MVP (M1–M4), refreshed when you open a PR or the inbox:**

- **"Reviewed" means** you submitted a review or clicked **Mark reviewed**. Opening a PR never counts.
- On first sight of a PR, `last_reviewed_sha` comes from the commit of your latest submitted review on GitHub, so existing PRs and other machines start correctly.
- **"Since last review" diff**: an interdiff (`git diff <lastReviewed>..<newHead>`). If history was rewritten, it falls back to `git range-diff`.
- New and unseen comments are highlighted. "Mark all seen" is available.

**In M5:**

- **Poller**: polls REST `/notifications` with conditional requests, obeying `X-Poll-Interval`, as the cheap change signal. GraphQL then runs only for PRs that changed. GraphQL has no ETag support, so it can't be the polling path. CI changes may not appear in notifications, so a slower GraphQL check covers your open PRs.
- **Tray**: closing the window leaves Docent in the tray, with a setting to quit instead.
- **OS notifications** with per-event toggles: new review requests, new commits on PRs you've reviewed, comments mentioning or replying to you, CI failed on your PR, your PR approved or changes requested. Clicking one opens the PR.
- Sync with GitHub's **"Viewed" file checkboxes** (GraphQL `markFileAsViewed`).

### 5.4 PR review view (M1–M4)

```
┌ File tree (status, viewed ✓, finding counts) ┬ Diff ───────────────────────┬ Agent panel ─────────┐
│                                               │  gutter: ● findings         │ [Explain] [Bugs]     │
│                                               │          💬 comments        │ [Style] [Modernize]  │
│                                               │  hover line → ? at far right│ [Chat]               │
└───────────────────────────────────────────────┴─────────────────────────────┴──────────────────────┘
 Top bar: PR title · branch → base · CI · "3 new commits since your review" · [Submit review ▾]
```

- Overview tab: description, checks, timeline, and an AI summary from the explainer lens.
- Diff modes: unified or split, full PR or since-last-review, ignore-whitespace.
- Show/hide findings by lens and severity.
- You can click and move the cursor anywhere freely. Agents are only triggered by the `?`.

### 5.5 Agents ("lenses") (M2–M3)

All lenses run through agent sessions with `cwd` set to the PR's worktree, so the agent can read anything in the repo using its own tools.

| Lens | Trigger | Input Docent provides | Output |
|---|---|---|---|
| **Explain** | The `?` on a line, plus a PR summary on open | PR title/body, the hunk, blame and log, the PR that introduced the code, Jira keys, related-change hints | Markdown: *what* it does, *why* it's there, **related changes** as clickable links, callers/impact |
| **Bugs** | Auto-run rules below | Full diff + PR context | JSON findings: correctness, edge cases, security, concurrency, error handling |
| **Style** | Auto-run rules | Diff + repo conventions (`CONTRIBUTING.md`, `AGENTS.md`, `CLAUDE.md`, `.editorconfig`) | JSON findings citing the in-repo example |
| **Modernize & lint** | Auto-run rules | Diff + detected language/framework versions + linter output, when available | JSON findings: newer idioms, deprecated APIs, linter violations, each with a suggested replacement |
| **Chat** | Manual | Current focus + prior panel content | Free-form follow-up in the same session |

**Explain behaviour**: a `?` appears at the far right of the hovered line, and clicking it triggers Explain. Explain runs per **hunk or enclosing symbol**, and line-specific questions are answered from that cached result. One Explain runs at a time per PR, and clicking elsewhere cancels the older request.

**"Why" context**: agents have no forge access, so Docent looks up the PR behind each blamed commit (with its linked issues) and includes it in the prompt, cached.

**Jira context**: Docent extracts keys like `ABC-123` from commit messages, PR titles and branch names, shows them as links, and passes them to the agent, which looks them up through its Jira MCP server. If there's no key, or the profile has no Jira tools, Explain carries on without it.

**Auto-run rules**: lenses run when you open a PR at a new head SHA, if all of these hold:
- The repo, branch or PR is trusted.
- The PR is open and not a draft.
- It requests your review or you pinned it.
- It's within 1,500 changed lines and 50 files (configurable), not counting lockfiles and generated files.

Otherwise lenses are on demand. Each lens uses one session per PR on the whole diff.

**New commits**: after a push, lenses re-run on the interdiff only, and findings on unchanged code carry forward. Each finding has a fingerprint (lens, file, normalised code, category), so dismissals survive new commits.

**Related changes**: Docent pre-computes cheap structural hints (other hunks touching the same symbols via ripgrep and optional tree-sitter, co-changed files, test files). The agent confirms or expands them.

**Structured output**: each lens prompt ends with a required JSON block (schema in the lens frontmatter). Docent validates it, retries once with the validation error, and otherwise shows the raw markdown. Every agent-reported line is checked against the real file, and a finding that fails is marked **unanchored**.

**Caching and cost control**: results are cached in SQLite keyed by `(lens, lens_hash, profile, repo, head_sha, base_sha, scope)`, so editing a lens or switching agents invalidates old results. There's a concurrency limit per profile, a cancel button (`session/cancel`), and a per-PR "run agents" toggle.

**Customizing lenses**: the four defaults ship as editable files in your config folder. Trusted repos can add overrides in `.docent/lenses/`, read from the **default branch only**, never from a PR head.

**Usage display**: per-lens usage is shown where the agent reports it (ACP `usage_update`).

### 5.6 Trust and agent safety (M2)

**Threat model**: PR content is attacker-controlled input. Code comments and `AGENTS.md` can carry prompt injection. Repo agent config (such as `.claude/settings.json` hooks) can run code when an agent starts. Linters such as `eslint` and `cargo clippy` execute repo code. The requirement: **opening an untrusted PR can't execute its code.**

**Two states:**

- **Untrusted** (default): code and diffs only. No agents, linters or repo config.
- **Trusted**: everything runs under the rules in this plan.

**Granting trust**: clicking `?` in an untrusted repo opens a dialog with two choices, and Docent remembers the answer:

- **Trust repo and all future branches**: covers all branches and future commits. Later PRs and branches auto-run under the rules in 5.5.
- **Trust this branch only**: resets on each new push.

Fork PRs always need **Trust this PR**, even in a trusted repo, and that resets on each push. Trust is stored locally and can be revoked in Settings.

**What trust means**: ACP permission requests are optional for agents, and not advertising `fs/write_text_file` doesn't stop an agent's built-in tools. So read-only is **best-effort**, not guaranteed, and the Trust dialog says so plainly. Controls:

- **Kiro**: a restricted custom agent with a tool allowlist.
- **Claude Code**: `settingSources: []`, a restricted `tools` list, `strictMcpConfig`, `disableAllHooks`, and an explicit `session/set_mode`.
- **All agents**: forge tokens stripped from the environment at spawn; Docent denies any permission request outside the policy.
- **MCP tools**: denied unless on the profile's allowlist, including any new tools a server adds later. Lens sessions get Jira **read** tools only (get, search), so injected content can't write to Jira.
- A full OS sandbox around the agent process is decided at M8, before public release.

**Linters**: Docent detects and runs linters on changed files, in trusted repos only, and never installs dependencies. A linter that can't run shows "skipped: dependencies missing", and Modernize continues without it.

**Transparency and retention**: every transcript (prompts, tool calls, permission decisions) is viewable in an "Agent activity" drawer. Transcripts are kept for 30 days (configurable) or until their worktree is removed. Findings are kept. Settings has a **Clear all** button.

### 5.7 Worktree lifecycle (M1)

- Fetch PR heads, including `refs/pull/N/head` for forks, into `refs/docent/*` only. Old SHAs stay reachable for interdiffs after a force-push.
- Check out in detached HEAD, so your branches stay clean.
- Delete a worktree when its PR closes or merges, and enforce a disk cap with least-recently-used eviction.

### 5.8 Commenting and submitting reviews (M4)

- Click the gutter or select lines → comment box (markdown, preview, GitHub `suggestion` blocks).
- **Promote a finding** to a comment in one click. Docent pre-fills the text and you edit it before posting.
- Comments collect into a **pending review**, then go out together with **Comment / Approve / Request changes**.
- Reply to, resolve and unresolve existing threads. Single comments can also post immediately.
- Optional: "draft with agent". You write the gist and the explainer lens polishes it, never posting automatically.

### 5.9 Explore mode (M6, M6b)

**M6:**

- Pick any bound repo, then a branch, tag or commit. Docent creates or refreshes a worktree at that ref.
- File tree and read-only code viewer with the same `?` agent panel. Explain uses blame and log ("introduced in #1234 to fix …").
- Bugs, Style and Modernize run on demand for the current file or selection.
- If the selected branch has an open PR, Docent offers to jump into Review mode.
- **Findings**: save, or copy as markdown.
- **Self-review**: run all lenses on your own branch before opening a PR.
- **Trace lens**: pick a line and a direction, either "how do we get here" or "where does this data come from". It returns validated JSON steps, shown as a vertical path of code snippets. Each step links to its source, and any step whose location doesn't check out is flagged. LSP or tree-sitter accuracy can be added later.
- **Bookmarks**: a sidebar panel of saved locations, stored locally. Each stores repo, file, line and a snippet of surrounding code, opens on the default branch by re-locating the snippet, and is flagged with a link to the original commit if the snippet can't be found.

**M6b, Projects:**

- A Project is a named set of bound repos, each at its own ref. The main use case is a service and the infrastructure repo that deploys it.
- Docent puts the worktrees side by side in one parent folder and uses that as `cwd`, which should work with any agent.
- Every repo in a Project must be trusted before agents run.
- Trace works across repos. Links between repos are usually matches on names (image tags, routes, env vars), so those steps are marked **inferred**.
- Explore only for now, not PR review.

Built from the start so this doesn't need a rework: `AgentBackend` takes multiple roots, and every finding and trace step stores its repo.

### 5.10 Jira (after M6b)

- **Create issue** and **comment on ticket** from a finding, through the agent's Jira MCP server.
- You edit and approve the exact text in Docent first. Docent then sends it in a new single-purpose session with an empty working folder, no repo content, and only the matching write tool plus search for duplicate checks.
- Docent shows the resulting ticket key and link.
- Profiles without Jira tools don't show the option.

### 5.11 GitLab (0.2)

- Implement `Forge` for GitLab: MR inbox, MR diffs and versions, discussions, draft notes, approvals.
- Self-managed GitLab via base URL. Personal access token first, OAuth (PKCE) later.

---

## 6. Data model (SQLite, initial)

```
accounts(id, forge, base_url, login, auth_source, keychain_ref NULL)
repos(id, account_id, owner, name, default_branch, local_path NULL, watched BOOL, profile_id NULL)
trust(id, repo_id, scope, ref NULL, pr_id NULL, head_sha NULL, granted_at)   -- scope: repo | branch | pr
pull_requests(id, repo_id, number, title, author, state, is_draft, is_fork, head_sha, base_sha, updated_at, raw_json)
review_state(pr_id, last_reviewed_sha, last_seen_event_at, viewed_files_json)
events(id, pr_id, kind, actor, created_at, seen BOOL)          -- commits, comments, reviews, CI
agent_profiles(id, name, command, args_json, env_json, mcp_allowlist_json)
account_profiles(account_id, profile_id)
lens_runs(id, lens, lens_hash, profile_id, pr_id NULL, project_id NULL, repo_id, head_sha, base_sha, scope, status,
          started_at, finished_at, transcript_path, transcript_expires_at)
findings(id, lens_run_id, repo_id, fingerprint, file, line_start, line_end, anchored BOOL, severity, category,
         title, body_md, suggestion, dismissed BOOL, saved BOOL)
pending_comments(id, pr_id, file, line, side, body_md, from_finding_id NULL)
bookmarks(id, repo_id, file, line, snippet, origin_sha, label)
projects(id, name)
project_repos(project_id, repo_id, ref)
```

---

## 7. Milestones

Each milestone is a set of short-lived branches off `main`.

| # | Milestone | Outcome |
|---|---|---|
| **M0.5** | **Work-laptop proof** (gate) | A plain Tauri window, built from source on the work laptop: token from `gh`, one watched repo's PRs, worktree, plain diff, one Explain through `kiro-cli acp`. Also tests a malicious `.kiro/` folder, Jira tools under the restricted Kiro agent, and MCP injection (for information only). |
| **M0** | **Viewer spike** | Monaco vs CodeMirror 6 on macOS: gutter decorations, hover `?` at the far right, large-file performance, plus a one-off WebKitGTK load test. Pick one. |
| **M1** | GitHub inbox + read-only PR view | Auth, watched repos, Bind/Clone, inbox, PR overview, diff viewer, worktree lifecycle |
| **M2** | Explain + trust + agent settings | Trust model, profiles and detection, tool policy, Explain, PR summary, Chat, activity drawer |
| **M3** | Review lenses | Bugs, Style, Modernize & lint; findings in gutter and file tree; fingerprints; caching; lens files |
| **M4** | Commenting, reviews, since-last-review | Pending review, promote finding, submit, threads, review state, interdiff/range-diff, new-comment highlighting |
| **M5** | Background tracking | Poller, tray, OS notifications, viewed-file sync |
| **M6** | Explore | Any-ref worktrees, file browser, on-demand lenses, saved findings, self-review, Trace, bookmarks |
| **M6b** | Projects | Multi-repo view and cross-repo Trace |
| **Jira** | Ticket writes | Create issue and comment via the agent's MCP server |
| **M8** | Release 0.1 | GitHub issues from findings, device-flow auth, full OS build matrix, paid signing (macOS notarized, Windows signed, Linux AppImage/deb/rpm), auto-update, OS sandbox decision, docs site, contributor guide |
| **0.2** | GitLab | Forge implementation |

**M0.5 gate:** M1 can't start until the Explain answer is useful on real work code and you decide it's worth continuing. If Kiro is blocked, swap the `AgentBackend` implementation or get the waiver first. If Explain isn't useful without ticket text, fix the Jira path before M1.

**MVP cut (personal use at work)**: M0.5–M4. **First public release**: through M8.

**Early builds** are unsigned with the workaround documented. Tauri's updater has its own signing key, so auto-update doesn't need paid certificates.

---

## 8. Testing strategy

- **Rust**: unit tests per crate. `docent-git` tests run against temp repos built in-test. `docent-forge` uses recorded HTTP fixtures (`wiremock`) and a fake `Forge`.
- **Mock ACP agent**: a tiny test binary that speaks ACP and replays scripted responses (including permission requests and malformed JSON), so the lens pipeline is tested without real agents or cost.
- **Hostile-repo fixtures**: repos with malicious `.kiro/`, `.claude/`, `.mcp.json`, `.docent/lenses/` and linter configs, proving nothing runs before trust.
- **Frontend**: Vitest + Vue Test Utils for components and stores. Playwright for a few smoke flows against the Vite dev build with mocked `invoke`.
- **CI**: GitHub Actions on macOS arm64 (`cargo fmt/clippy/test`, `pnpm lint/test`, `tauri build`), plus a cheap Linux `cargo check`. The full OS matrix arrives at M8.

## 9. Open-source hygiene

- MIT license (already present). Add `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, issue/PR templates, `SECURITY.md` (token handling, trust model, agent limits).
- `AGENTS.md` / `CLAUDE.md` describing repo conventions.
- Conventional commits + `release-please` for changelogs and versioning.
- No telemetry by default.

## 10. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Kiro can't be run over ACP at work | M0.5 gate, `AgentBackend` interface, waiver request |
| Agents ignore the read-only policy | Trust gate, restricted agent configs, stripped tokens, plain wording in the Trust dialog, OS sandbox decision at M8 |
| Kiro loads hostile workspace config under ACP (unconfirmed) | Malicious `.kiro/` test in M0.5 |
| Prompt injection writes to Jira | Read-only tool allowlist in lens sessions, separate approved session for writes |
| ACP agents differ in capabilities and output quality | Schema validation with retry, per-agent quirk flags in profiles, mock-agent tests |
| Agent cost and latency on big PRs | Auto-run limits, interdiff re-runs, cache, cancel |
| Viewer problems on Linux WebKitGTK surface late | One-off load test in M0, full matrix at M8 |
| GitHub rate limits from polling | Conditional `/notifications` polling, GraphQL only on change, backoff |
| `/notifications` doesn't accept the `gh` token | Test in M0.5; fall back to a slower GraphQL poll |
| Force-pushes break "since last review" | `git range-diff` fallback; old SHAs kept in `refs/docent/*` |
| Name overlaps with `TransluceAI/docent`, and npm/PyPI names are taken | Accepted; no package needs those names. Trademark search before M8 |
